use crate::common::{IpFamily, OutputFormat};
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

    match format_enum {
        OutputFormat::Txt => {
            // IPv4
            let ipv4_file = format!("IPv4_{}.txt", safe_code);
            let header_v4 = make_header(&now_str, &safe_code, "N/A");
            write_list_txt(&ipv4_file, ipv4_list, &header_v4).await?;

            // IPv6
            let ipv6_file = format!("IPv6_{}.txt", safe_code);
            let header_v6 = make_header(&now_str, &safe_code, "N/A");
            write_list_txt(&ipv6_file, ipv6_list, &header_v6).await?;
        }
        OutputFormat::Nft => {
            // IPv4
            let ipv4_file = format!("IPv4_{}.nft", safe_code);
            let header_v4 = make_header(&now_str, &safe_code, "N/A");
            write_list_nft(&ipv4_file, ipv4_list, &header_v4).await?;

            // IPv6
            let ipv6_file = format!("IPv6_{}.nft", safe_code);
            let header_v6 = make_header(&now_str, &safe_code, "N/A");
            write_list_nft(&ipv6_file, ipv6_list, &header_v6).await?;
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

    match format_enum {
        OutputFormat::Txt => {
            let file_name = format!("AS_{}_{}.txt", safe_as, family.as_str());
            write_list_txt(&file_name, ipnets, &header).await?;
            println!(
                "[output] Wrote TXT for AS_{} {}",
                safe_as,
                family.as_str()
            );
        }
        OutputFormat::Nft => {
            let file_name = format!("AS_{}_{}.nft", safe_as, family.as_str());
            write_list_nft(&file_name, ipnets, &header).await?;
            println!(
                "[output] Wrote NFT for AS_{} {}",
                safe_as,
                family.as_str()
            );
        }
    }
    Ok(())
}

/// 国コード+AS番号の重複CIDRリストを書き出す
pub async fn write_overlap_to_file(
    country_code: &str,
    as_number: &str,
    overlaps: &BTreeSet<IpNet>,
    format_enum: OutputFormat,
) -> Result<(), AppError> {
    use chrono::Local;
    let safe_cc = sanitize_identifier(country_code);
    let safe_as = sanitize_identifier(as_number);

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

    if overlaps_v4.is_empty() && overlaps_v6.is_empty() {
        println!(
            "[overlap] No overlap found for country={} and AS={}",
            country_code, as_number
        );
        return Ok(());
    }

    match format_enum {
        OutputFormat::Txt => {
            if !overlaps_v4.is_empty() {
                let filename_v4 = format!("overlap_{}_{}_IPv4.txt", safe_cc, safe_as);
                let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let header_v4 = make_header(&now_str, &safe_cc, &safe_as);
                write_list_txt(&filename_v4, &overlaps_v4, &header_v4).await?;
            }
            if !overlaps_v6.is_empty() {
                let filename_v6 = format!("overlap_{}_{}_IPv6.txt", safe_cc, safe_as);
                let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let header_v6 = make_header(&now_str, &safe_cc, &safe_as);
                write_list_txt(&filename_v6, &overlaps_v6, &header_v6).await?;
            }
        }
        OutputFormat::Nft => {
            if !overlaps_v4.is_empty() {
                let filename_v4 = format!("overlap_{}_{}_IPv4.nft", safe_cc, safe_as);
                let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let header_v4 = make_header(&now_str, &safe_cc, &safe_as);
                write_list_nft(&filename_v4, &overlaps_v4, &header_v4).await?;
            }
            if !overlaps_v6.is_empty() {
                let filename_v6 = format!("overlap_{}_{}_IPv6.nft", safe_cc, safe_as);
                let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let header_v6 = make_header(&now_str, &safe_cc, &safe_as);
                write_list_nft(&filename_v6, &overlaps_v6, &header_v6).await?;
            }
        }
    }

    Ok(())
}
