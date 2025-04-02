use crate::{output::write_ip_lists_to_files, parse::parse_ip_lines};
use ipnet::IpNet;
use std::collections::BTreeSet;

/// 指定された国コードと、ダウンロード済みのRIRファイル文字列から、
/// IPアドレスをパースしてファイル書き込みまで実行する。
pub async fn process_country_code(
    country_code: &str,
    rir_texts: &[String],
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ipv4_set, ipv6_set) = parse_and_collect_ips(country_code, rir_texts)?;
    // 結果をファイルに書き出す
    write_ip_lists_to_files(country_code, &ipv4_set, &ipv6_set, mode)?;
    Ok(())
}

/// 全RIRテキストから、指定国コードに合致するIPアドレスをすべて集約し、BTreeSetとして返す。
pub fn parse_and_collect_ips(
    country_code: &str,
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn std::error::Error + Send + Sync>> {
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
