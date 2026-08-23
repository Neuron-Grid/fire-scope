use crate::common::{IpFamily, OutputFormat, aggregate_ipnets, debug_log};
use crate::constants::MAX_JSON_DOWNLOAD_BYTES;
use crate::error::AppError;
use crate::fetch::fetch_json_with_limit;
use crate::output::write_as_ip_list_to_file;
use futures::future::join_all;
use ipnet::IpNet;
use reqwest::Client;
use serde_json::Value;
use std::{collections::BTreeSet, str::FromStr, sync::Arc};
use tokio::sync::Semaphore;

type AsProcessOutcome = (String, Result<(), AppError>);

/// AS の発表プレフィックスを複数ソースから取得する（RIPEstat 優先、ARIN RDAP をフォールバック）
/// RPKI検証なし
pub async fn get_prefixes_via_rdap(
    client: &Client,
    as_number: &str,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    // 1) RIPEstat announced-prefixes API
    let nets = match fetch_ripe_stat_prefixes(client, as_number).await {
        Ok(mut nets) => {
            // フォールバックとして ARIN も併合（失敗は無視）
            if let Ok(mut arin) = fetch_arin_originas_prefixes(client, as_number).await {
                nets.append(&mut arin);
            }
            nets
        }
        Err(e) => {
            debug_log(format!("RIPEstat fetch failed for AS{}: {}", as_number, e));
            // 2) ARIN OriginAS RDAP（米地域中心、非網羅）
            fetch_arin_originas_prefixes(client, as_number).await?
        }
    };
    Ok(dedup_and_partition(&nets))
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

/// RIPEstat: Announced Prefixes API から CIDR を抽出
async fn fetch_ripe_stat_prefixes(
    client: &Client,
    as_number: &str,
) -> Result<Vec<IpNet>, AppError> {
    // https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS{asn}
    let url = format!(
        "https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS{}",
        as_number
    );
    let json: Value = fetch_json_with_limit(client, &url, MAX_JSON_DOWNLOAD_BYTES).await?;
    let mut nets = Vec::new();
    if let Some(prefixes) = json
        .get("data")
        .and_then(|d| d.get("prefixes"))
        .and_then(|p| p.as_array())
    {
        for obj in prefixes {
            if let Some(net) = obj
                .get("prefix")
                .and_then(|v| v.as_str())
                .and_then(|pfx| IpNet::from_str(pfx).ok())
            {
                nets.push(net);
            }
        }
    }
    Ok(nets)
}

/// ARIN 独自 RDAP OriginAS ネットワーク API
async fn fetch_arin_originas_prefixes(
    client: &Client,
    as_number: &str,
) -> Result<Vec<IpNet>, AppError> {
    let base = "https://rdap.arin.net/registry";
    let url = format!("{base}/arin_originas0_networksbyoriginas/{as_number}");
    let json: Value = fetch_json_with_limit(client, &url, MAX_JSON_DOWNLOAD_BYTES).await?;
    extract_prefixes_from_arin(&json)
}

/// Vec<IpNet> → (IPv4, IPv6) 集合に分割し aggregate
fn dedup_and_partition(nets: &[IpNet]) -> (BTreeSet<IpNet>, BTreeSet<IpNet>) {
    aggregate_ipnets(nets.iter().copied())
}

/// 複数 AS を並列取得してファイル出力
pub async fn process_as_numbers(
    client: &Client,
    as_numbers: &[String],
    output_format: OutputFormat,
    concurrency: usize,
) -> Result<(), AppError> {
    let max_concurrent = if concurrency == 0 { 1 } else { concurrency };
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let handles = as_numbers
        .iter()
        .map(|asn| {
            let asn_cloned = asn.clone();
            let fmt_c = output_format;
            let client_c = client.clone();
            let sem_c = semaphore.clone();
            tokio::spawn(async move {
                let _permit = sem_c.acquire_owned().await?;
                let result = async {
                    let (v4, v6) = get_prefixes_via_rdap(&client_c, &asn_cloned).await?;
                    write_ip_list(&asn_cloned, IpFamily::V4, &v4, fmt_c).await?;
                    write_ip_list(&asn_cloned, IpFamily::V6, &v6, fmt_c).await?;
                    Ok::<(), AppError>(())
                }
                .await;
                Ok::<AsProcessOutcome, AppError>((asn_cloned, result))
            })
        })
        .collect::<Vec<_>>();

    let outcomes = join_all(handles)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect::<Result<Vec<_>, AppError>>()?;
    outcomes
        .iter()
        .filter_map(|(asn, result)| result.as_ref().err().map(|error| (asn, error)))
        .for_each(|(asn, error)| {
            eprintln!("Warning: failed to process AS{asn}: {error}");
        });

    finalize_as_processing(outcomes)
}

/// ファイル書き出しヘルパ
async fn write_ip_list(
    as_number: &str,
    ip_family: IpFamily,
    ip_set: &BTreeSet<IpNet>,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    if ip_set.is_empty() {
        debug_log(format!(
            "No {} routes for {}",
            ip_family.as_str(),
            as_number
        ));
    } else {
        write_as_ip_list_to_file(as_number, ip_family, ip_set, output_format).await?;
    }
    Ok(())
}

fn finalize_as_processing(outcomes: Vec<AsProcessOutcome>) -> Result<(), AppError> {
    let failures = outcomes
        .into_iter()
        .filter_map(|(asn, result)| result.err().map(|e| format!("AS{}: {}", asn, e)))
        .collect::<Vec<_>>();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Other(format!(
            "Failed to process {} AS number(s): {}",
            failures.len(),
            failures.join("; ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::finalize_as_processing;
    use crate::error::AppError;

    #[test]
    fn finalize_as_processing_succeeds_when_all_asns_succeed() {
        let outcomes = vec![
            ("1234".to_string(), Ok::<(), AppError>(())),
            ("5678".to_string(), Ok::<(), AppError>(())),
        ];

        assert!(finalize_as_processing(outcomes).is_ok());
    }

    #[test]
    fn finalize_as_processing_fails_when_any_asn_fails() {
        let outcomes = vec![
            ("1234".to_string(), Ok::<(), AppError>(())),
            ("5678".to_string(), Err(AppError::Other("boom".into()))),
        ];

        let err = match finalize_as_processing(outcomes) {
            Ok(()) => panic!("expected partial failure"),
            Err(err) => err,
        };

        match err {
            AppError::Other(msg) => {
                assert!(msg.contains("Failed to process 1 AS number(s)"));
                assert!(msg.contains("AS5678: boom"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn finalize_as_processing_fails_when_all_asns_fail() {
        let outcomes = vec![
            ("1234".to_string(), Err(AppError::Other("timeout".into()))),
            (
                "5678".to_string(),
                Err(AppError::Other("rdap unavailable".into())),
            ),
        ];

        let err = match finalize_as_processing(outcomes) {
            Ok(()) => panic!("expected total failure"),
            Err(err) => err,
        };

        match err {
            AppError::Other(msg) => {
                assert!(msg.contains("Failed to process 2 AS number(s)"));
                assert!(msg.contains("AS1234: timeout"));
                assert!(msg.contains("AS5678: rdap unavailable"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
