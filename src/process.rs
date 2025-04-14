use crate::common::OutputFormat;
use crate::error::AppError;
use crate::output::write_ip_lists_to_files;
use crate::parse::{parse_all_country_codes, parse_ip_lines};
use ipnet::IpNet;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub async fn process_country_code(
    country_code: &str,
    rir_texts: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let (ipv4_set, ipv6_set) = parse_and_collect_ips(country_code, rir_texts)?;
    write_ip_lists_to_files(country_code, &ipv4_set, &ipv6_set, mode, output_format).await?;
    Ok(())
}

pub fn parse_and_collect_ips(
    country_code: &str,
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut ipv4_vec = Vec::new();
    let mut ipv6_vec = Vec::new();

    // RIRファイル群から、指定された国コードに該当するIPを収集
    for text in rir_texts {
        let (v4, v6) = parse_ip_lines(text, country_code)?;
        ipv4_vec.extend(v4);
        ipv6_vec.extend(v6);
    }

    // 取得したIPv4/IPv6をソート
    ipv4_vec.sort();
    ipv6_vec.sort();

    // 同一・隣接するプレフィックスをまとめる
    let agg_v4 = IpNet::aggregate(&ipv4_vec);
    let agg_v6 = IpNet::aggregate(&ipv6_vec);

    // BTreeSet にまとめる
    // 重複除去や順序保持のため
    let ipv4_set = agg_v4.into_iter().collect::<BTreeSet<_>>();
    let ipv6_set = agg_v6.into_iter().collect::<BTreeSet<_>>();

    Ok((ipv4_set, ipv6_set))
}

/// マップから特定の国コードを抽出して書き出す
pub async fn process_country_code_from_map(
    country_code: &str,
    country_map: &HashMap<String, (Vec<IpNet>, Vec<IpNet>)>,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let upper_code = country_code.to_uppercase();

    let (ipv4_vec, ipv6_vec) = match country_map.get(&upper_code) {
        Some(ip_lists) => ip_lists,
        None => {
            eprintln!(
                "No IP address corresponding to the country code could be found: {}",
                upper_code
            );
            return Ok(());
        }
    };

    let mut ipv4_sorted = ipv4_vec.clone();
    let mut ipv6_sorted = ipv6_vec.clone();
    ipv4_sorted.sort();
    ipv6_sorted.sort();

    let ipv4_set = ipv4_sorted.into_iter().collect::<BTreeSet<_>>();
    let ipv6_set = ipv6_sorted.into_iter().collect::<BTreeSet<_>>();

    write_ip_lists_to_files(&upper_code, &ipv4_set, &ipv6_set, mode, output_format).await?;
    Ok(())
}

/// 全ての国コードに対するIPアドレスをまとめてパースし、指定された国コードのみ書き出す
pub async fn process_all_country_codes(
    country_codes: &[String],
    rir_texts: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let country_map = parse_all_country_codes(rir_texts)?;

    let country_map = Arc::new(country_map);

    let mut tasks: Vec<JoinHandle<Result<(), AppError>>> = Vec::new();
    for code in country_codes {
        let code_clone = code.clone();
        let mode_clone = mode.to_string();
        let format_clone = output_format;
        let map_arc = Arc::clone(&country_map);

        let handle = tokio::spawn(async move {
            process_country_code_from_map(&code_clone, &map_arc, &mode_clone, format_clone).await
        });
        tasks.push(handle);
    }

    for t in tasks {
        t.await??;
    }

    Ok(())
}
