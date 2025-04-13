use crate::common::{IpFamily, OutputFormat};
use crate::output::write_as_ip_list_to_file;
use ipnet::IpNet;
use std::{collections::BTreeSet, error::Error, process::Stdio, sync::Arc};
use tokio::{process::Command, sync::Semaphore};

/// 1つのAS番号に対し、whoisを1回だけ実行し、IPv4/IPv6ルートを同時取得して返す。
pub async fn get_ips_for_as_once(
    as_number: &str,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn Error + Send + Sync>> {
    let output = Command::new("whois")
        .arg("-h")
        .arg("whois.radb.net")
        .arg("--")
        .arg(format!("-i origin {}", as_number))
        .stderr(Stdio::inherit())
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("whois command failed for {}", as_number).into());
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);

    // ここでIPv4, IPv6に仕分け
    let mut v4s = BTreeSet::new();
    let mut v6s = BTreeSet::new();

    for line in stdout_str.lines() {
        if line.starts_with("route:") {
            // 例: "route: 192.0.2.0/24"
            if let Some(ip_str) = line.split_whitespace().nth(1) {
                if let Ok(ip) = ip_str.parse::<IpNet>() {
                    // ip.is_ipv4() の代わりにパターンマッチ
                    if let IpNet::V4(_) = ip {
                        v4s.insert(ip);
                    }
                }
            }
        } else if line.starts_with("route6:") {
            // 例: "route6: 2001:db8::/32"
            if let Some(ip_str) = line.split_whitespace().nth(1) {
                if let Ok(ip) = ip_str.parse::<IpNet>() {
                    if let IpNet::V6(_) = ip {
                        v6s.insert(ip);
                    }
                }
            }
        }
    }

    // 必要に応じてサブネットをまとめる
    let aggregated_v4 = IpNet::aggregate(&v4s.iter().copied().collect::<Vec<_>>());
    let aggregated_v6 = IpNet::aggregate(&v6s.iter().copied().collect::<Vec<_>>());

    Ok((
        aggregated_v4.into_iter().collect(),
        aggregated_v6.into_iter().collect(),
    ))
}

/// 複数の AS番号を並列で処理し、IPv4/IPv6リストをファイル出力する。
/// 同時実行数はSemaphoreで制限。
pub async fn process_as_numbers(
    as_numbers: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 同時に叩くwhoisコマンドの上限。必要に応じて調整
    let max_concurrent = 5;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut handles = Vec::new();

    for as_number in as_numbers {
        let as_number = as_number.clone();
        let mode = mode.to_string();
        let format = output_format;
        let sem_clone = semaphore.clone();

        // 各ASを並行処理
        let handle = tokio::spawn(async move {
            // セマフォで同時実行数を制限
            let _permit = sem_clone.acquire_owned().await?;

            match get_ips_for_as_once(&as_number).await {
                Ok((v4set, v6set)) => {
                    // IPv4/IPv6をファイルに書き出す
                    if v4set.is_empty() {
                        println!("[asn] No IPv4 routes found for {}", as_number);
                    } else {
                        write_as_ip_list_to_file(&as_number, IpFamily::V4, &v4set, &mode, format)
                            .await?;
                    }

                    if v6set.is_empty() {
                        println!("[asn] No IPv6 routes found for {}", as_number);
                    } else {
                        write_as_ip_list_to_file(&as_number, IpFamily::V6, &v6set, &mode, format)
                            .await?;
                    }
                }
                Err(e) => eprintln!("[asn] Error processing {}: {}", as_number, e),
            };

            Ok::<(), Box<dyn Error + Send + Sync>>(())
        });
        handles.push(handle);
    }

    // 全タスクが完了するのを待機
    for h in handles {
        h.await??;
    }

    Ok(())
}
