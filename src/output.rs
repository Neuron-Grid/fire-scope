use crate::common::{IpFamily, OutputFormat, debug_log};
use crate::error::AppError;
use crate::output_common::{make_header, sanitize_identifier, write_list_nft, write_list_txt};
use chrono::Local;
use ipnet::IpNet;
use std::collections::BTreeSet;

/// IPv4/IPv6リストをファイルに書き出す
/// 国コード用
pub async fn write_ip_lists_to_files(
    country_code: &str,
    ipv4_list: &BTreeSet<IpNet>,
    ipv6_list: &BTreeSet<IpNet>,
    format_enum: OutputFormat,
) -> Result<(), AppError> {
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let safe_code = sanitize_identifier(&country_code.to_uppercase());
    let header = make_header(&now_str, &safe_code, "N/A");
    let ext = format_enum.extension();
    let v4_file = format!("IPv4_{}.{}", safe_code, ext);
    let v6_file = format!("IPv6_{}.{}", safe_code, ext);

    match format_enum {
        OutputFormat::Txt => {
            write_list_txt(&v4_file, ipv4_list, &header).await?;
            write_list_txt(&v6_file, ipv6_list, &header).await?;
        }
        OutputFormat::Nft => {
            write_list_nft(&v4_file, ipv4_list, &header).await?;
            write_list_nft(&v6_file, ipv6_list, &header).await?;
        }
    }
    Ok(())
}

/// IPv4/IPv6リストをファイルに書き出す
/// AS番号用
pub async fn write_as_ip_list_to_file(
    as_number: &str,
    family: IpFamily,
    ipnets: &BTreeSet<IpNet>,
    format_enum: OutputFormat,
) -> Result<(), AppError> {
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let safe_as = sanitize_identifier(as_number);
    let header = make_header(&now_str, "N/A", &safe_as);

    let ext = format_enum.extension();
    let file_name = format!("AS_{}_{}.{}", safe_as, family.as_str(), ext);
    match format_enum {
        OutputFormat::Txt => write_list_txt(&file_name, ipnets, &header).await?,
        OutputFormat::Nft => write_list_nft(&file_name, ipnets, &header).await?,
    }
    debug_log(format!("Wrote {} for AS_{} {}", ext.to_uppercase(), safe_as, family.as_str()));
    Ok(())
}

/// 国コード+AS番号の重複CIDRリストを書き出す
pub async fn write_overlap_to_file(
    country_code: &str,
    as_number: &str,
    overlaps: &BTreeSet<IpNet>,
    format_enum: OutputFormat,
) -> Result<(), AppError> {
    let safe_cc = sanitize_identifier(country_code);
    let safe_as = sanitize_identifier(as_number);

    let overlaps_v4: BTreeSet<IpNet> = overlaps
        .iter()
        .copied()
        .filter(|net| matches!(net, IpNet::V4(_)))
        .collect();

    let overlaps_v6: BTreeSet<IpNet> = overlaps
        .iter()
        .copied()
        .filter(|net| matches!(net, IpNet::V6(_)))
        .collect();

    if overlaps_v4.is_empty() && overlaps_v6.is_empty() {
        debug_log(format!(
            "No overlap found for country={} and AS={}",
            country_code, as_number
        ));
        return Ok(());
    }

    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header = make_header(&now_str, &safe_cc, &safe_as);
    let ext = format_enum.extension();

    for (label, set) in [("IPv4", &overlaps_v4), ("IPv6", &overlaps_v6)] {
        if set.is_empty() {
            continue;
        }
        let filename = format!("overlap_{}_{}_{}.{}", safe_cc, safe_as, label, ext);
        match format_enum {
            OutputFormat::Txt => write_list_txt(&filename, set, &header).await?,
            OutputFormat::Nft => write_list_nft(&filename, set, &header).await?,
        }
    }

    Ok(())
}
