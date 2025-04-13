use crate::common::OutputFormat;
use crate::output::write_ip_lists_to_files; // これが async に
use crate::parse::{parse_all_country_codes, parse_ip_lines};
use ipnet::IpNet;
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// 指定された国コードと、ダウンロード済みのRIRファイル文字列から
/// IPアドレスをパースしてファイル書き込みまで実行する
pub async fn process_country_code(
    country_code: &str,
    rir_texts: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (ipv4_set, ipv6_set) = parse_and_collect_ips(country_code, rir_texts)?;
    // 結果をファイルに書き出す
    write_ip_lists_to_files(country_code, &ipv4_set, &ipv6_set, mode, output_format).await?;
    Ok(())
}

/// 全RIRテキストから、指定国コードに合致するIPアドレスをすべて集約し、BTreeSetとして返す
pub fn parse_and_collect_ips(
    country_code: &str,
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn Error + Send + Sync>> {
    let mut ipv4_vec = Vec::new();
    let mut ipv6_vec = Vec::new();

    for text in rir_texts {
        match parse_ip_lines(text, country_code) {
            Ok((v4, v6)) => {
                ipv4_vec.extend(v4);
                ipv6_vec.extend(v6);
            }
            Err(e) => eprintln!(
                "[parse_and_collect_ips] Error parsing for country '{}': {}",
                country_code, e
            ),
        }
    }

    ipv4_vec.sort();
    ipv6_vec.sort();

    let ipv4_set = ipv4_vec.into_iter().collect::<BTreeSet<_>>();
    let ipv6_set = ipv6_vec.into_iter().collect::<BTreeSet<_>>();

    Ok((ipv4_set, ipv6_set))
}

/// パース済みの国コードマップから特定の国コードのIPアドレスを取得し、ファイルに書き出す
pub async fn process_country_code_from_map(
    country_code: &str,
    country_map: &HashMap<String, (Vec<IpNet>, Vec<IpNet>)>,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let upper_code = country_code.to_uppercase();

    // マップから該当する国コードのIPアドレスを取得
    let (ipv4_vec, ipv6_vec) = match country_map.get(&upper_code) {
        Some(ip_lists) => ip_lists,
        None => {
            eprintln!(
                "No IP address corresponding to the country code could be found.\n{}",
                upper_code
            );
            return Ok(());
        }
    };

    // ソートとBTreeSetへの変換
    let mut ipv4_sorted = ipv4_vec.clone();
    let mut ipv6_sorted = ipv6_vec.clone();
    ipv4_sorted.sort();
    ipv6_sorted.sort();

    let ipv4_set = ipv4_sorted.into_iter().collect::<BTreeSet<_>>();
    let ipv6_set = ipv6_sorted.into_iter().collect::<BTreeSet<_>>();

    // 非同期ファイル出力
    write_ip_lists_to_files(&upper_code, &ipv4_set, &ipv6_set, mode, output_format).await?;

    Ok(())
}

/// 全RIRテキストから全ての国コードに対するIPアドレスをパースし、
/// 指定された国コードのリストに対応するIPアドレスをファイルに書き出す
pub async fn process_all_country_codes(
    country_codes: &[String],
    rir_texts: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 全ての国コードに対するIPアドレスを一度にパース
    let country_map = match parse_all_country_codes(rir_texts) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("Country code parsing failed.\n{}", e);
            return Err(e);
        }
    };

    let country_map = Arc::new(country_map);

    // 各国コードに対して非同期タスクを起動
    let mut tasks: Vec<JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>> = Vec::new();
    for code in country_codes {
        let code_clone = code.clone();
        let mode_clone = mode.to_string();
        let format_clone = output_format;
        let map_arc = Arc::clone(&country_map);

        let handle = tokio::spawn(async move {
            if let Err(e) =
                process_country_code_from_map(&code_clone, &map_arc, &mode_clone, format_clone)
                    .await
            {
                eprintln!("Error (country={}): {}", code_clone, e);
            }
            Ok(())
        });
        tasks.push(handle);
    }

    // すべてのタスクが完了するのを待つ
    for t in tasks {
        let _ = t.await?;
    }

    Ok(())
}
