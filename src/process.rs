use crate::common::OutputFormat;
use crate::error::AppError;
use crate::output::write_ip_lists_to_files;
use ipnet::{IpNet, Ipv6Net};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// 単一国コードを処理し、結果ファイルを出力
pub async fn process_country_code(
    country_code: &str,
    rir_texts: &[String],
    output_format: OutputFormat,
) -> Result<(), AppError> {
    // CPUバウンド部をTokioのブロッキングスレッドにオフロード
    let (ipv4_set, ipv6_set) =
        tokio::task::block_in_place(|| parse_and_collect_ips(country_code, rir_texts))?;

    // I/Oはasyncのまま
    write_ip_lists_to_files(country_code, &ipv4_set, &ipv6_set, output_format).await
}

/// 全RIRテキストから該当国コードのIP一覧を集約し、そのまま書き出し
pub async fn process_all_country_codes(
    country_codes: &[String],
    rir_texts: &[String],
    output_format: OutputFormat,
) -> Result<(), AppError> {
    // 1回だけ全RIRテキストをパースして国コード→(IPv4,IPv6)のマップを作る（CPU重）
    let rir_texts_owned = rir_texts.to_owned();
    let country_map = tokio::task::spawn_blocking(move || {
        crate::parse::parse_all_country_codes(&rir_texts_owned)
    })
    .await??;
    let country_map_arc = Arc::new(country_map);

    // 国コードごとに並列タスクを生成（事前パース結果を参照）
    let mut tasks: Vec<JoinHandle<Result<(), AppError>>> = Vec::new();
    for code in country_codes {
        let code_cloned = code.clone();
        let map_cloned = Arc::clone(&country_map_arc);
        tasks.push(tokio::spawn(async move {
            crate::process::process_country_code_from_map(&code_cloned, &map_cloned, output_format)
                .await
        }));
    }

    // すべてのタスクを待機
    for handle in tasks {
        handle.await??;
    }
    Ok(())
}

/// IPv4範囲をBTreeSetへ直接挿入
/// 逐次集合化する
fn insert_ipv4_range(
    start_str: &str,
    value_str: &str,
    set: &mut BTreeSet<IpNet>,
) -> Result<(), AppError> {
    let cidrs = crate::ipv4_utils::parse_ipv4_range_to_cidrs(start_str, value_str)?;
    for net in cidrs {
        set.insert(net);
    }
    Ok(())
}

/// RIR テキストを1行ずつストリーミング解析し、重複排除しながら集合化
/// 戻り値は **重複無し・昇順** の `BTreeSet`
pub fn parse_and_collect_ips(
    country_code: &str,
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut ipv4_set = BTreeSet::<IpNet>::new();
    let mut ipv6_set = BTreeSet::<IpNet>::new();
    let cc_upper = country_code.to_ascii_uppercase();

    for text in rir_texts {
        for line in text.lines() {
            // コメントやreserved行はスキップ
            if line.starts_with('#') || line.contains('*') || line.contains("reserved") {
                continue;
            }
            let params: Vec<&str> = line.split('|').collect();
            if params.len() < 5 || !params[1].eq_ignore_ascii_case(&cc_upper) {
                continue;
            }

            match params[2] {
                "ipv4" => insert_ipv4_range(params[3], params[4], &mut ipv4_set)?,
                "ipv6" => {
                    let cidr = format!("{}/{}", params[3], params[4]);
                    let net = cidr
                        .parse::<Ipv6Net>()
                        .map_err(|e| AppError::ParseError(format!("Ipv6 parse error: {e}")))?;
                    ipv6_set.insert(IpNet::V6(net));
                }
                _ => {}
            }
        }
    }

    // 逐次集合化である程度集約済みだが、さらに最終aggregateで最小化
    let agg_v4 = IpNet::aggregate(&ipv4_set.iter().copied().collect::<Vec<_>>());
    let agg_v6 = IpNet::aggregate(&ipv6_set.iter().copied().collect::<Vec<_>>());

    Ok((agg_v4.into_iter().collect(), agg_v6.into_iter().collect()))
}

pub async fn process_country_code_from_map(
    country_code: &str,
    country_map: &HashMap<String, (Vec<IpNet>, Vec<IpNet>)>,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let upper = country_code.to_ascii_uppercase();
    let (v4_vec, v6_vec) = match country_map.get(&upper) {
        Some(tup) => tup,
        None => {
            eprintln!("No IPs found for country code: {}", upper);
            return Ok(());
        }
    };

    // CPU バウンドの aggregate を block_in_place で分離
    let (ipv4_set, ipv6_set) = tokio::task::block_in_place(|| {
        let v4_set = IpNet::aggregate(&v4_vec).into_iter().collect();
        let v6_set = IpNet::aggregate(&v6_vec).into_iter().collect();
        (v4_set, v6_set)
    });

    write_ip_lists_to_files(&upper, &ipv4_set, &ipv6_set, output_format).await
}
