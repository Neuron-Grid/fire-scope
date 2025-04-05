use crate::common::{IpFamily, OutputFormat};
use crate::output::write_as_ip_list_to_file;
use ipnet::IpNet;
use std::{collections::BTreeSet, error::Error, process::Stdio};
use tokio::process::Command;

/// AS番号とIPバージョン(IPv4/IPv6)を指定してWHOISサーバからルート情報を取得し、
/// IPアドレスの集合を返す。（重複除外+ソートのため BTreeSet）
pub async fn get_ips_for_as(
    as_number: &str,
    family: IpFamily,
) -> Result<BTreeSet<IpNet>, Box<dyn Error + Send + Sync>> {
    // WHOISコマンド: whois -h whois.radb.net -- -i origin ASxxxx
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

    // route_key = "route:" or "route6:"
    let route_key = family.route_key();

    // イテレータを使った抽出
    let ipnets: Vec<IpNet> = stdout_str
        .lines()
        .filter_map(|line| {
            if line.contains(route_key) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                parts
                    .get(1)
                    .and_then(|cidr_str| cidr_str.parse::<IpNet>().ok())
            } else {
                None
            }
        })
        .collect();

    // 集約してBTreeSetへ格納
    let aggregated = IpNet::aggregate(&ipnets);
    Ok(aggregated.into_iter().collect())
}

/// 複数のAS番号を受け取り、それぞれIPv4/IPv6のWHOISルート情報を取得して出力ファイルに書き込む。
/// main.rsから呼び出す想定のエントリポイント。
pub async fn process_as_numbers(
    as_numbers: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 同一AS番号に対して IPv4, IPv6 を順次処理
    for as_number in as_numbers {
        for &family in &[IpFamily::V4, IpFamily::V6] {
            match get_ips_for_as(as_number, family).await {
                Ok(set) => {
                    if set.is_empty() {
                        println!(
                            "[asn] No {} routes found for {}",
                            family.as_str(),
                            as_number
                        );
                    } else {
                        // nft/txtの出力切り替え
                        write_as_ip_list_to_file(as_number, family, &set, mode, output_format)?;
                    }
                }
                Err(e) => eprintln!(
                    "[asn] Error processing {} ({}): {}",
                    as_number,
                    family.as_str(),
                    e
                ),
            }
        }
    }

    Ok(())
}
