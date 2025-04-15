use crate::common::{IpFamily, OutputFormat};
use crate::error::AppError;
use crate::output::write_as_ip_list_to_file;
use ipnet::IpNet;
use reqwest::Client;
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::Semaphore;

#[derive(serde::Deserialize)]
struct RipeStatAnnouncedPrefixes {
    data: AnnouncedPrefixesData,
}

#[derive(serde::Deserialize)]
struct AnnouncedPrefixesData {
    prefixes: Vec<AnnouncedPrefix>,
}

#[derive(serde::Deserialize)]
struct AnnouncedPrefix {
    prefix: String,
}

/// 1つのAS番号に対応するBGPルートを取得
/// RIPEstat APIを使用
/// RPKI検証なし
pub async fn get_ips_for_as_once_no_rpki(
    client: &Client,
    as_number: &str,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let url = format!(
        "https://stat.ripe.net/data/announced-prefixes/data.json?resource={}",
        as_number
    );
    let body = client.get(&url).send().await?.text().await?;
    let parsed: RipeStatAnnouncedPrefixes = serde_json::from_str(&body)
        .map_err(|e| AppError::ParseError(format!("Failed to parse RIPEstat JSON: {e}")))?;

    // 入力データからパース・集約する部分を分離し、イミュータブルデータの流れを明確化
    let (v4s, v6s) = partition_and_parse_ipnets(&parsed.data.prefixes);

    // 集約
    let aggregated_v4 = IpNet::aggregate(&v4s.iter().copied().collect::<Vec<_>>());
    let aggregated_v6 = IpNet::aggregate(&v6s.iter().copied().collect::<Vec<_>>());

    Ok((
        aggregated_v4.into_iter().collect(),
        aggregated_v6.into_iter().collect(),
    ))
}

/// 複数のAS番号を指定し BGPルートを並行取得+ファイル出力
/// RPKI検証なし
pub async fn process_as_numbers_no_rpki(
    client: &Client,
    as_numbers: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let max_concurrent = 5;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    // 非同期タスクを作成
    let handles = as_numbers
        .iter()
        .map(|as_number| {
            let as_number_clone = as_number.clone();
            let mode_clone = mode.to_string();
            let format_clone = output_format;
            let sem_clone = semaphore.clone();
            let client_clone = client.clone();

            tokio::spawn(async move {
                let _permit = sem_clone.acquire_owned().await?;
                match get_ips_for_as_once_no_rpki(&client_clone, &as_number_clone).await {
                    Ok((v4set, v6set)) => {
                        // IPv4をファイル出力orログ表示
                        write_ip_list(
                            &as_number_clone,
                            IpFamily::V4,
                            &v4set,
                            &mode_clone,
                            format_clone,
                        )
                        .await?;

                        // IPv6をファイル出力orログ表示
                        write_ip_list(
                            &as_number_clone,
                            IpFamily::V6,
                            &v6set,
                            &mode_clone,
                            format_clone,
                        )
                        .await?;
                    }
                    Err(e) => eprintln!("Error processing {as_number_clone}: {e}"),
                };
                Ok::<(), AppError>(())
            })
        })
        .collect::<Vec<_>>();

    // タスクの完了を待機
    for handle in handles {
        handle.await??;
    }

    Ok(())
}

/// AnnouncedPrefix のリストからIpNetをパースし、IPv4とIPv6を分割して返す
fn partition_and_parse_ipnets(prefixes: &[AnnouncedPrefix]) -> (BTreeSet<IpNet>, BTreeSet<IpNet>) {
    // イテレータとfilter_mapでエラーを無視しつつパース
    let ipnets = prefixes
        .iter()
        .filter_map(|pfx| pfx.prefix.parse::<IpNet>().ok());

    // partitionは(集めたい要素, それ以外の要素)に分割する
    let mut v4s = BTreeSet::new();
    let mut v6s = BTreeSet::new();

    for ipnet in ipnets {
        match ipnet {
            IpNet::V4(_) => {
                v4s.insert(ipnet);
            }
            IpNet::V6(_) => {
                v6s.insert(ipnet);
            }
        }
    }
    (v4s, v6s)
}

/// 指定されたIPアドレスリストをファイルに書き出す。
/// 空の場合はログ表示のみを行う。
async fn write_ip_list(
    as_number: &str,
    ip_family: IpFamily,
    ip_set: &BTreeSet<IpNet>,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    if ip_set.is_empty() {
        match ip_family {
            IpFamily::V4 => {
                println!("No IPv4 routes for {as_number}");
            }
            IpFamily::V6 => {
                println!("No IPv6 routes for {as_number}");
            }
        }
    } else {
        write_as_ip_list_to_file(as_number, ip_family, ip_set, mode, output_format).await?;
    }
    Ok(())
}
