use crate::common::{IpFamily, OutputFormat};
use crate::error::AppError;
use crate::output::write_as_ip_list_to_file;
use ipnet::IpNet;
use reqwest::Client;
use serde_json::Value;
use std::{collections::BTreeSet, str::FromStr, sync::Arc};
use tokio::sync::Semaphore;

/// RDAPから AS → (IPv4, IPv6) プレフィックスを取得する
/// RPKI検証なし
pub async fn get_prefixes_via_rdap(
    client: &Client,
    as_number: &str,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut nets: Vec<IpNet> = Vec::new();

    // RDAP を1か所のみ問い合わせる（例: ARIN）
    let base = "https://rdap.arin.net/registry";
    let url = format!("{base}/arin_originas0_networksbyoriginas/{as_number}");

    let resp = client.get(&url).send().await?;
    if resp.status().is_success() {
        let json: Value = resp.json().await?;
        nets.extend(extract_prefixes_from_arin(&json)?);
    }

    // ※ RPKI でフィルタせずにそのまま使う
    let (v4set, v6set) = dedup_and_partition(&nets);
    Ok((v4set, v6set))
}

/// ARIN OriginAS RDAP 応答から CIDR を抽出
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

/// Vec<IpNet> → (IPv4, IPv6) 集合に分割し aggregate
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

/// 複数 AS を並列取得してファイル出力
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
