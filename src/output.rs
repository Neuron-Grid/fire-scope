use chrono::{Datelike, Local, Timelike};
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// ソート済みのIPv4/IPv6リストをファイルに書き出すモジュール。
/// すべてファイル出力で完結している。
pub fn write_ip_lists_to_files(
    country_code: &str,
    ipv4_list: &BTreeSet<IpNet>,
    ipv6_list: &BTreeSet<IpNet>,
    // 追記・上書きモードを引数で受け取る
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ipv4_file = format!("IPv4_{}.txt", country_code);
    let ipv6_file = format!("IPv6_{}.txt", country_code);

    // それぞれ書き込み処理を実行
    write_single_ip_list(&ipv4_file, ipv4_list, mode)?;
    write_single_ip_list(&ipv6_file, ipv6_list, mode)?;

    Ok(())
}

/// 1つのファイルに書き込むヘルパー関数
/// 書き込み先を変更したい場合は、この関数を差し替えるだけにする設計が可能。
fn write_single_ip_list<P: AsRef<Path>>(
    path: P,
    nets: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Local::now();
    let formatted_header = format!(
        "# {}年{}月{}日 {}時{}分\n",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute()
    );

    let lines: Vec<String> = nets.iter().map(|net| net.to_string()).collect();
    let content = format!("{}{}", formatted_header, lines.join("\n"));

    match mode {
        "append" => {
            // 追記モードでファイルを開く
            // 無ければ新規作成
            let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
            file.write_all(content.as_bytes())?;
            println!("[output] Appended IP list to: {}", path.as_ref().display());
        }
        _ => {
            // デフォルトは上書き
            fs::write(&path, &content)?;
            println!(
                "[output] Wrote (overwrite) IP list to: {}",
                path.as_ref().display()
            );
        }
    }

    Ok(())
}
