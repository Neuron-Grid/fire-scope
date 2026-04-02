use crate::common::{IpSetPair, IpVecPair, OutputFormat, debug_log};
use crate::error::AppError;
use crate::output::write_ip_lists_to_files;
use ipnet::IpNet;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tokio::task::JoinHandle;

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

/// 指定国コードの IP を RIR テキストから抽出し、集約・重複排除して返す
pub fn parse_and_collect_ips(
    country_code: &str,
    rir_texts: &[String],
) -> Result<IpSetPair, AppError> {
    let mut all_v4 = Vec::new();
    let mut all_v6 = Vec::new();
    for text in rir_texts {
        let (v4, v6) = crate::parse::parse_ip_lines(text, country_code)?;
        all_v4.extend(v4);
        all_v6.extend(v6);
    }
    let agg_v4 = IpNet::aggregate(&all_v4).into_iter().collect();
    let agg_v6 = IpNet::aggregate(&all_v6).into_iter().collect();
    Ok((agg_v4, agg_v6))
}

pub async fn process_country_code_from_map(
    country_code: &str,
    country_map: &HashMap<String, IpVecPair>,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let upper = country_code.to_ascii_uppercase();
    let (v4_vec, v6_vec) = match country_map.get(&upper) {
        Some(tup) => tup,
        None => {
            debug_log(format!("No IPs found for country code: {}", upper));
            return Ok(());
        }
    };

    // parse_all_country_codes で既に aggregate 済みなので変換のみ
    let ipv4_set: BTreeSet<IpNet> = v4_vec.iter().copied().collect();
    let ipv6_set: BTreeSet<IpNet> = v6_vec.iter().copied().collect();

    write_ip_lists_to_files(&upper, &ipv4_set, &ipv6_set, output_format).await
}
