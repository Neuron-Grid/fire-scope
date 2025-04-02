use crate::output::write_ip_lists_to_files;
use crate::parse::parse_ip_lines;
use ipnet::IpNet;
use std::collections::BTreeSet;

/// 指定された国コードと、ダウンロード済みのRIRファイル文字列から、
/// IPアドレスをパースしてファイル書き込みまで実行する。
pub async fn process_country_code(
    country_code: &str,
    rir_texts: &[String],
    // 追記・上書きモードを受け取る引数を追加
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // パース結果をまとめる
    // BTreeSetで重複排除+自動ソート
    let (ipv4_set, ipv6_set) = parse_and_collect_ips(country_code, rir_texts)?;

    // 結果をファイルに書き出す（mode引数を渡す）
    write_ip_lists_to_files(country_code, &ipv4_set, &ipv6_set, mode)?;

    Ok(())
}

/// 全RIRテキストから、指定国コードに合致するIPアドレスをすべて集約し、
/// BTreeSetとして返す。
fn parse_and_collect_ips(
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
            Err(e) => {
                eprintln!(
                    "[parse_and_collect_ips] Error parsing for country code '{}': {}",
                    country_code, e
                );
                // パース失敗時はスキップするが、必要に応じて Err で落としても良い。
            }
        }
    }

    // IpNetのOrd実装で昇順ソートされる
    ipv4_vec.sort();
    ipv6_vec.sort();

    // BTreeSetに格納
    // 重複排除 + 順序保持
    let ipv4_set = ipv4_vec.into_iter().collect::<BTreeSet<IpNet>>();
    let ipv6_set = ipv6_vec.into_iter().collect::<BTreeSet<IpNet>>();

    Ok((ipv4_set, ipv6_set))
}
