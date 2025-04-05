use crate::common::{IpFamily, OutputFormat};
use chrono::Local;
use ipnet::IpNet;
use std::{
    collections::BTreeSet,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// IPv4/IPv6リストをファイルに書き出す
/// 国コード用
pub fn write_ip_lists_to_files(
    country_code: &str,
    ipv4_list: &BTreeSet<IpNet>,
    ipv6_list: &BTreeSet<IpNet>,
    mode: &str,
    format_enum: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match format_enum {
        OutputFormat::Txt => {
            // 従来のテキストファイル出力
            let ipv4_file = format!("IPv4_{}.txt", country_code);
            let ipv6_file = format!("IPv6_{}.txt", country_code);
            write_single_ip_list_txt(&ipv4_file, ipv4_list, mode, country_code, "N/A")?;
            write_single_ip_list_txt(&ipv6_file, ipv6_list, mode, country_code, "N/A")?;
        }
        OutputFormat::Nft => {
            // nftables用ファイル出力
            let ipv4_file = format!("IPv4_{}.nft", country_code);
            let ipv6_file = format!("IPv6_{}.nft", country_code);

            write_single_ip_list_nft(&ipv4_file, ipv4_list, country_code, mode, IpFamily::V4)?;
            write_single_ip_list_nft(&ipv6_file, ipv6_list, country_code, mode, IpFamily::V6)?;
        }
    }
    Ok(())
}

/// テキストファイルへの書き込み
fn write_single_ip_list_txt<P: AsRef<Path>>(
    path: P,
    nets: &BTreeSet<IpNet>,
    mode: &str,
    country_code: &str,
    as_number: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header = format!(
        "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
        now_str, country_code, as_number
    );

    // 従来の本体部分
    let lines: Vec<String> = nets.iter().map(|net| net.to_string()).collect();
    let content = format!("{}{}\n", header, lines.join("\n"));

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

/// nftables用ファイルへの書き込み
/// 国コード用
fn write_single_ip_list_nft<P: AsRef<Path>>(
    path: P,
    nets: &BTreeSet<IpNet>,
    country_code: &str,
    mode: &str,
    _family: IpFamily,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use chrono::Local;

    let file_path = path.as_ref();
    let define_name = file_path
        .file_stem()
        .and_then(|os| os.to_str())
        .unwrap_or("unknown_define");
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header = format!(
        "# Generated at: {}\n# Country Code: {}\n# AS Number: N/A\n\n",
        now_str, country_code
    );

    // NFT出力内容
    let mut content = String::new();
    content.push_str(&header);
    content.push_str(&format!("define {} {{\n", define_name));
    for net in nets {
        content.push_str(&format!("    {},\n", net));
    }
    content.push_str("}\n");

    match mode {
        "append" => {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_path)?;
            file.write_all(content.as_bytes())?;
            println!("[output] Appended nft rules to: {}", file_path.display());
        }
        _ => {
            std::fs::write(&file_path, &content)?;
            println!(
                "[output] Wrote (overwrite) nft rules to: {}",
                file_path.display()
            );
        }
    }

    Ok(())
}

/// IPv4/IPv6リストをファイルに書き出す
/// AS番号用
pub fn write_as_ip_list_to_file(
    as_number: &str,
    family: IpFamily,
    ipnets: &BTreeSet<IpNet>,
    mode: &str,
    format_enum: OutputFormat,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match format_enum {
        OutputFormat::Txt => {
            // 従来のテキストファイル出力
            let file_name = format!("AS_{}_{}.txt", as_number, family.as_str());
            write_as_ip_list_txt(&file_name, ipnets, mode, as_number)?;
        }
        OutputFormat::Nft => {
            // nftables用出力
            let file_name = format!("AS_{}_{}.nft", as_number, family.as_str());
            write_as_ip_list_nft(&file_name, ipnets, as_number, mode, family)?;
        }
    }
    Ok(())
}

/// テキスト出力
/// AS番号用
fn write_as_ip_list_txt(
    file_name: &str,
    ipnets: &BTreeSet<IpNet>,
    mode: &str,
    as_number: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header = format!(
        "# Generated at: {}\n# Country Code: N/A\n# AS Number: {}\n\n",
        now_str, as_number
    );

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

/// nftables出力
/// AS番号用
fn write_as_ip_list_nft(
    file_name: &str,
    ipnets: &BTreeSet<IpNet>,
    as_number: &str,
    mode: &str,
    _family: IpFamily,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use chrono::Local;
    use std::path::Path;

    // 先頭コメント用のヘッダー
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header = format!(
        "# Generated at: {}\n# Country Code: N/A\n# AS Number: {}\n\n",
        now_str, as_number
    );

    // ファイル名からdefine名を作成
    let define_name = Path::new(file_name)
        .file_stem()
        .and_then(|os| os.to_str())
        .unwrap_or("unknown_define");

    // "define <define_name> {\n  <CIDR>,\n  <CIDR>,\n}"形式の文字列を組み立て
    let mut content = String::new();
    content.push_str(&header);
    content.push_str(&format!("define {} {{\n", define_name));
    for net in ipnets {
        content.push_str(&format!("    {},\n", net));
    }
    content.push_str("}\n");

    // appendかoverwriteに応じてファイル出力
    match mode {
        "append" => {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file_name)?;
            file.write_all(content.as_bytes())?;
            println!("[output] Appended nft rules to: {}", file_name);
        }
        _ => {
            std::fs::write(&file_name, &content)?;
            println!("[output] Wrote (overwrite) nft rules to: {}", file_name);
        }
    }

    Ok(())
}

/// 国コード+AS番号の重複CIDRリストを書き出す
/// Overlap
pub fn write_overlap_to_file(
    country_code: &str,
    as_number: &str,
    overlaps: &BTreeSet<IpNet>,
    mode: &str,
    format_enum: OutputFormat,
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

    match format_enum {
        OutputFormat::Txt => {
            // テキスト出力
            write_overlap_txt(country_code, as_number, &overlaps_v4, &overlaps_v6, mode)?;
        }
        OutputFormat::Nft => {
            // nftables出力
            write_overlap_nft(country_code, as_number, &overlaps_v4, &overlaps_v6, mode)?;
        }
    }

    Ok(())
}

/// txt出力
/// Overlaps
fn write_overlap_txt(
    country_code: &str,
    as_number: &str,
    overlaps_v4: &BTreeSet<IpNet>,
    overlaps_v6: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // IPv4
    if !overlaps_v4.is_empty() {
        let filename_v4 = format!("overlap_{}_{}_IPv4.txt", country_code, as_number);
        let header_v4 = format!(
            "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
            now_str, country_code, as_number
        );
        let body_v4 = overlaps_v4
            .iter()
            .map(|net| net.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let content_v4 = format!("{}{}\n", header_v4, body_v4);

        match mode {
            "append" => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filename_v4)?;
                file.write_all(content_v4.as_bytes())?;
                println!("[overlap] Appended IPv4 overlaps to: {}", filename_v4);
            }
            _ => {
                fs::write(&filename_v4, &content_v4)?;
                println!("[overlap] Wrote IPv4 overlaps to: {}", filename_v4);
            }
        }
    }

    // IPv6
    if !overlaps_v6.is_empty() {
        let filename_v6 = format!("overlap_{}_{}_IPv6.txt", country_code, as_number);
        let header_v6 = format!(
            "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
            now_str, country_code, as_number
        );
        let body_v6 = overlaps_v6
            .iter()
            .map(|net| net.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let content_v6 = format!("{}{}\n", header_v6, body_v6);

        match mode {
            "append" => {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filename_v6)?;
                file.write_all(content_v6.as_bytes())?;
                println!("[overlap] Appended IPv6 overlaps to: {}", filename_v6);
            }
            _ => {
                fs::write(&filename_v6, &content_v6)?;
                println!("[overlap] Wrote IPv6 overlaps to: {}", filename_v6);
            }
        }
    }

    Ok(())
}

/// nft出力
/// Overlaps
fn write_overlap_nft(
    country_code: &str,
    as_number: &str,
    overlaps_v4: &BTreeSet<IpNet>,
    overlaps_v6: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use chrono::Local;
    use std::path::Path;

    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // IPv4用
    if !overlaps_v4.is_empty() {
        let filename_v4 = format!("overlap_{}_{}_IPv4.nft", country_code, as_number);
        let define_name_v4 = Path::new(&filename_v4)
            .file_stem()
            .and_then(|os| os.to_str())
            .unwrap_or("unknown_define_v4");
        let header_v4 = format!(
            "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
            now_str, country_code, as_number
        );

        let mut content_v4 = String::new();
        content_v4.push_str(&header_v4);
        content_v4.push_str(&format!("define {} {{\n", define_name_v4));
        for net in overlaps_v4 {
            content_v4.push_str(&format!("    {},\n", net));
        }
        content_v4.push_str("}\n");

        match mode {
            "append" => {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filename_v4)?;
                file.write_all(content_v4.as_bytes())?;
                println!("[overlap] Appended IPv4 overlaps nft to: {}", filename_v4);
            }
            _ => {
                std::fs::write(&filename_v4, &content_v4)?;
                println!("[overlap] Wrote IPv4 overlaps nft to: {}", filename_v4);
            }
        }
    }

    // IPv6用
    if !overlaps_v6.is_empty() {
        let filename_v6 = format!("overlap_{}_{}_IPv6.nft", country_code, as_number);
        let define_name_v6 = Path::new(&filename_v6)
            .file_stem()
            .and_then(|os| os.to_str())
            .unwrap_or("unknown_define_v6");
        let header_v6 = format!(
            "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
            now_str, country_code, as_number
        );

        let mut content_v6 = String::new();
        content_v6.push_str(&header_v6);
        content_v6.push_str(&format!("define {} {{\n", define_name_v6));
        for net in overlaps_v6 {
            content_v6.push_str(&format!("    {},\n", net));
        }
        content_v6.push_str("}\n");

        match mode {
            "append" => {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&filename_v6)?;
                file.write_all(content_v6.as_bytes())?;
                println!("[overlap] Appended IPv6 overlaps nft to: {}", filename_v6);
            }
            _ => {
                std::fs::write(&filename_v6, &content_v6)?;
                println!("[overlap] Wrote IPv6 overlaps nft to: {}", filename_v6);
            }
        }
    }

    Ok(())
}
