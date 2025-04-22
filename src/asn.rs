use crate::common::{IpFamily, OutputFormat};
use crate::constants::RDAP_BASE_URLS;
use crate::error::AppError;
use crate::output::write_as_ip_list_to_file;
use crate::rpki::rpki_filter::filter_valid_by_rpki;
use ipnet::IpNet;
use reqwest::Client;
use serde_json::Value;
use std::{collections::BTreeSet, str::FromStr, sync::Arc};
use tokio::sync::Semaphore;

/// RDAPでAS → (IPv4, IPv6)プレフィックスを取得し、RPKI VALIDのみ返す
pub async fn get_prefixes_via_rdap(
    client: &Client,
    as_number: &str,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut nets: Vec<IpNet> = Vec::new();

    // RDAP を順に問い合わせ
    for base in RDAP_BASE_URLS {
        // ARINはOriginAS拡張を優先
        let url = if base.contains("arin.net") {
            format!("{base}/arin_originas0_networksbyoriginas/{as_number}")
        } else {
            format!("{base}/autnum/{as_number}")
        };

        let resp = match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let json: Value = resp.json().await?;
        if base.contains("arin.net") {
            nets.extend(extract_prefixes_from_arin(&json)?);
        } else {
            nets.extend(extract_prefixes_generic(&json)?);
        }
        // 取得できたら他のRDAPはスキップ
        if !nets.is_empty() {
            break;
        }
    }

    // RPKI VALIDのみ残す
    let nets_valid = filter_rpki_valid(as_number, &nets).await?;
    let (v4set, v6set) = dedup_and_partition(&nets_valid);
    Ok((v4set, v6set))
}

/// Vec<IpNet>をRPKI VALIDのみ抽出
async fn filter_rpki_valid(as_number: &str, nets: &[IpNet]) -> Result<Vec<IpNet>, AppError> {
    let asn_num = as_number.trim_start_matches("AS").parse::<u32>()?;
    filter_valid_by_rpki(asn_num, nets).await
}

/// ARIN OriginAS RDAP応答からCIDR抽出
fn extract_prefixes_from_arin(v: &Value) -> Result<Vec<IpNet>, AppError> {
    let mut nets = Vec::new();
    if let Some(arr) = v
        .get("arin_originas0_networkSearchResults")
        .and_then(|v| v.as_array())
    {
        for obj in arr {
            let (prefix_key, len_key) = match obj.get("ipVersion").and_then(|v| v.as_str()) {
                Some("v4") => ("v4prefix", "length"),
                Some("v6") => ("v6prefix", "length"),
                _ => continue,
            };
            if let (Some(prefix), Some(len)) = (
                obj.get(prefix_key).and_then(|v| v.as_str()),
                obj.get(len_key),
            ) {
                let cidr = format!("{}/{}", prefix, len);
                if let Ok(net) = IpNet::from_str(&cidr) {
                    nets.push(net);
                }
            }
        }
    }
    Ok(nets)
}

/// 汎用RDAP(cidr0 拡張)応答からCIDR抽出
fn extract_prefixes_generic(v: &Value) -> Result<Vec<IpNet>, AppError> {
    let mut nets = Vec::new();
    if let Some(arr) = v.get("cidr0_cidrs").and_then(|v| v.as_array()) {
        for obj in arr {
            let key = if obj.get("v4prefix").is_some() {
                "v4prefix"
            } else {
                "v6prefix"
            };
            if let (Some(prefix), Some(len)) =
                (obj.get(key).and_then(|v| v.as_str()), obj.get("length"))
            {
                let cidr = format!("{}/{}", prefix, len);
                if let Ok(net) = IpNet::from_str(&cidr) {
                    nets.push(net);
                }
            }
        }
    }
    Ok(nets)
}

/// Vec<IpNet> → (IPv4, IPv6)集合に分割しaggregate
fn dedup_and_partition(nets: &[IpNet]) -> (BTreeSet<IpNet>, BTreeSet<IpNet>) {
    let agg = IpNet::aggregate(&nets.to_vec());
    let mut v4 = BTreeSet::new();
    let mut v6 = BTreeSet::new();
    for net in agg {
        match net {
            IpNet::V4(_) => {
                v4.insert(net);
            }
            IpNet::V6(_) => {
                v6.insert(net);
            }
        }
    }
    (v4, v6)
}

/// 複数ASを並列取得してファイル出力
pub async fn process_as_numbers(
    client: &Client,
    as_numbers: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let max_concurrent = 5;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let handles = as_numbers
        .iter()
        .map(|asn| {
            let asn_cloned = asn.clone();
            let mode_c = mode.to_string();
            let fmt_c = output_format;
            let client_c = client.clone();
            let sem_c = semaphore.clone();
            tokio::spawn(async move {
                let _permit = sem_c.acquire_owned().await?;
                match get_prefixes_via_rdap(&client_c, &asn_cloned).await {
                    Ok((v4, v6)) => {
                        write_ip_list(&asn_cloned, IpFamily::V4, &v4, &mode_c, fmt_c).await?;
                        write_ip_list(&asn_cloned, IpFamily::V6, &v6, &mode_c, fmt_c).await?;
                    }
                    Err(e) => eprintln!("Error processing {asn_cloned}: {e}"),
                };
                Ok::<(), AppError>(())
            })
        })
        .collect::<Vec<_>>();

    for h in handles {
        h.await??;
    }
    Ok(())
}

/// ファイル書き出しヘルパ
async fn write_ip_list(
    as_number: &str,
    ip_family: IpFamily,
    ip_set: &BTreeSet<IpNet>,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    if ip_set.is_empty() {
        println!("No {} routes for {}", ip_family.as_str(), as_number);
    } else {
        write_as_ip_list_to_file(as_number, ip_family, ip_set, mode, output_format).await?;
    }
    Ok(())
}
