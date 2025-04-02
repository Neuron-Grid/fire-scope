use crate::common::IpFamily;
use chrono::{Datelike, Local, Timelike};
use ipnet::IpNet;
use std::{
    collections::BTreeSet,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// ソート済みのIPv4/IPv6リストをファイルに書き出すモジュール。
/// すべてファイル出力で完結している。
pub fn write_ip_lists_to_files(
    country_code: &str,
    ipv4_list: &BTreeSet<IpNet>,
    ipv6_list: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ipv4_file = format!("IPv4_{}.txt", country_code);
    let ipv6_file = format!("IPv6_{}.txt", country_code);

    write_single_ip_list(&ipv4_file, ipv4_list, mode)?;
    write_single_ip_list(&ipv6_file, ipv6_list, mode)?;

    Ok(())
}

/// 1つのファイルに書き込むヘルパー関数
fn write_single_ip_list<P: AsRef<Path>>(
    path: P,
    nets: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Local::now();
    let formatted_header = format!(
        "# {}/{}/{} {}:{}\n",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute()
    );

    // ipnet -> string 変換して結合
    let lines: Vec<String> = nets.iter().map(|net| net.to_string()).collect();
    let content = format!("{}{}\n", formatted_header, lines.join("\n"));

    match mode {
        "append" => {
            let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
            file.write_all(content.as_bytes())?;
            println!("[output] Appended IP list to: {}", path.as_ref().display());
        }
        _ => {
            fs::write(&path, &content)?;
            println!(
                "[output] Wrote (overwrite) IP list to: {}",
                path.as_ref().display()
            );
        }
    }

    Ok(())
}

/// AS番号+IPファミリ用のルートリストをファイルに書き出す関数.
/// 「AS_XXXX_YYYY.txt」という形式でファイルを生成する.
pub fn write_as_ip_list_to_file(
    as_number: &str,
    family: IpFamily,
    ipnets: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    let file_name = format!("AS_{}_{}.txt", as_number, family.as_str());

    let header = format!("# Execution Date and Time: {}\n", now);
    let body = ipnets
        .iter()
        .map(IpNet::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!("{}{}\n", header, body);

    match mode {
        "append" => {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_name)?;
            file.write_all(content.as_bytes())?;
            println!("[output] Appended IP list to: {}", file_name);
        }
        _ => {
            fs::write(&file_name, content)?;
            println!("[output] Wrote (overwrite) IP list to: {}", file_name);
        }
    }

    Ok(())
}
