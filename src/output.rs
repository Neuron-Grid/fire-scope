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
) -> Result<(), Box<dyn Error + Send + Sync>> {
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
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = Local::now();
    let formatted_header = format!(
        "# {}/{}/{} {}:{}\n",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute()
    );

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

/// 国コード & AS番号の重複CIDRリストをファイルに書き出すヘルパー関数
pub fn write_overlap_to_file(
    country_code: &str,
    as_number: &str,
    overlaps: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // IPv4 / IPv6 を仕分け
    let overlaps_v4: BTreeSet<IpNet> = overlaps
        .iter()
        .cloned()
        .filter(|net| matches!(net, IpNet::V4(_)))
        .collect();

    let overlaps_v6: BTreeSet<IpNet> = overlaps
        .iter()
        .cloned()
        .filter(|net| matches!(net, IpNet::V6(_)))
        .collect();

    // 重複がなければファイル出力しない
    if overlaps_v4.is_empty() && overlaps_v6.is_empty() {
        println!(
            "[overlap] No overlap found for country={} and AS={}",
            country_code, as_number
        );
        return Ok(());
    }

    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

    // IPv4 の書き出し
    if !overlaps_v4.is_empty() {
        let filename_v4 = format!("overlap_{}_{}_IPv4.txt", country_code, as_number);
        let header_v4 = format!(
            "# Overlap (IPv4) between Country={} and AS={} at {}\n",
            country_code, as_number, now_str
        );
        let body_v4 = overlaps_v4
            .iter()
            .map(|net| net.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let content_v4 = format!("{}\n{}\n", header_v4, body_v4);

        match mode {
            "append" => {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filename_v4)?;
                file.write_all(content_v4.as_bytes())?;
                println!("[overlap] Appended IPv4 overlaps to: {}", filename_v4);
            }
            _ => {
                std::fs::write(&filename_v4, &content_v4)?;
                println!("[overlap] Wrote IPv4 overlaps to: {}", filename_v4);
            }
        }
    }

    // IPv6 の書き出し
    if !overlaps_v6.is_empty() {
        let filename_v6 = format!("overlap_{}_{}_IPv6.txt", country_code, as_number);
        let header_v6 = format!(
            "# Overlap (IPv6) between Country={} and AS={} at {}\n",
            country_code, as_number, now_str
        );
        let body_v6 = overlaps_v6
            .iter()
            .map(|net| net.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let content_v6 = format!("{}\n{}\n", header_v6, body_v6);

        match mode {
            "append" => {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filename_v6)?;
                file.write_all(content_v6.as_bytes())?;
                println!("[overlap] Appended IPv6 overlaps to: {}", filename_v6);
            }
            _ => {
                std::fs::write(&filename_v6, &content_v6)?;
                println!("[overlap] Wrote IPv6 overlaps to: {}", filename_v6);
            }
        }
    }

    Ok(())
}
