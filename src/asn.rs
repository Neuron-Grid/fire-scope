use chrono::Local;
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::error::Error;
use std::process::Stdio;
use tokio::process::Command;

/// AS番号とIPバージョン(IPv4/IPv6)を指定してWHOISサーバからルート情報を取得し、
/// IPアドレスの集合を返す（重複除外+ソートのため BTreeSet）。
async fn get_ips_for_as(
    as_number: &str,
    version: &str,
) -> Result<BTreeSet<IpNet>, Box<dyn Error + Send + Sync>> {
    // route_key は Python版に倣って IPv4 => "route:", IPv6 => "route6:"
    let route_key = match version {
        "IPv4" => "route:",
        "IPv6" => "route6:",
        _ => return Err("Invalid IP version. Must be 'IPv4' or 'IPv6'.".into()),
    };

    // WHOISコマンド: whois -h whois.radb.net -- -i origin ASxxxx
    // tokio::process::Commandを使用
    let output = Command::new("whois")
        .arg("-h")
        .arg("whois.radb.net")
        .arg("--")
        .arg(format!("-i origin {}", as_number))
        // エラー出力を継承しておく
        .stderr(Stdio::inherit())
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("whois command failed for {}", as_number).into());
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);

    // 行単位でフィルタして「route:」「route6:」を含む行のみ抽出
    // 例: "route: 192.0.2.0/24" のような行からCIDR文字列を取り出す
    let mut ipnets = Vec::new();
    for line in stdout_str.lines() {
        if line.contains(route_key) {
            // ["route:", "192.0.2.0/24"] のように分割想定
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let cidr_str = parts[1];
            // ipnetパッケージを使ってCIDRとしてパース
            if let Ok(net) = cidr_str.parse::<IpNet>() {
                ipnets.push(net);
            }
        }
    }

    // まとめて集約。IpNet::aggregateで重複・オーバーラップをマージ
    let aggregated = IpNet::aggregate(&ipnets);

    // BTreeSetへ格納（重複除外 + ソート）
    Ok(aggregated.into_iter().collect())
}

/// 取得した IPv4/IPv6 の BTreeSet をファイルに書き込む。
/// ファイル名は `AS_{as_number}_{version}.txt` のように生成。
fn write_as_ip_list_to_file(
    as_number: &str,
    version: &str,
    ipnets: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    let file_name = format!("AS_{}_{}.txt", as_number, version);
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();

    // ヘッダ行: Python版同様に日時を入れる
    let mut content = format!("# Execution Date and Time: {}\n", now);
    for net in ipnets {
        content.push_str(&format!("{}\n", net));
    }

    match mode {
        "append" => {
            // 追記モード
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_name)?;
            file.write_all(content.as_bytes())?;
            println!("[asn] Appended IP list to: {}", file_name);
        }
        _ => {
            // デフォルト上書き
            fs::write(&file_name, &content)?;
            println!("[asn] Wrote (overwrite) IP list to: {}", file_name);
        }
    }

    Ok(())
}

/// 複数のAS番号を受け取り、それぞれIPv4/IPv6のWHOISルート情報を取得して出力ファイルに書き込む。
/// main.rsから呼び出す想定のエントリポイント。
pub async fn process_as_numbers(
    as_numbers: &[String],
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // すべて並行で取る場合は tokio::spawn するが、
    // 連続で良いなら順次forループでもOK。
    // 下記では「同一ASでIPv4/IPv6を連続、次のASへ…」という流れにしている。
    for as_number in as_numbers {
        for version in ["IPv4", "IPv6"] {
            match get_ips_for_as(as_number, version).await {
                Ok(set) => {
                    if set.is_empty() {
                        println!("[asn] No {} routes found for {}", version, as_number);
                    } else {
                        write_as_ip_list_to_file(as_number, version, &set, mode)?;
                    }
                }
                Err(e) => eprintln!("[asn] Error processing {} ({}): {}", as_number, version, e),
            }
        }
    }

    Ok(())
}
