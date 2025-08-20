use crate::common::OutputFormat;
use crate::error::AppError;
use crate::output::write_ip_lists_to_files;
use ipnet::IpNet;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tokio::task::JoinHandle;
use crate::common::debug_log;

/// 単一国コードを処理し、結果ファイルを出力
pub async fn process_country_code(
    country_code: &str,
    rir_texts: &[String],
    output_format: OutputFormat,
) -> Result<(), AppError> {
    // 実装統一: すべての国コードを一度にパースしてから取り出す
    let rir_texts_owned = rir_texts.to_owned();
    let upper = country_code.to_ascii_uppercase();
    let (ipv4_set, ipv6_set) = tokio::task::spawn_blocking(move || {
        let map = crate::parse::parse_all_country_codes(&rir_texts_owned)?;
        let (v4_vec, v6_vec) = map.get(&upper).cloned().unwrap_or_default();
        let v4_set: BTreeSet<IpNet> = v4_vec.into_iter().collect();
        let v6_set: BTreeSet<IpNet> = v6_vec.into_iter().collect();
        Ok::<_, AppError>((v4_set, v6_set))
    })
    .await??;

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
// 旧実装の逐次パースヘルパは廃止（共通パーサへ統一）

/// RIR テキストを1行ずつストリーミング解析し、重複排除しながら集合化
/// 戻り値は **重複無し・昇順** の `BTreeSet`
pub fn parse_and_collect_ips(
    country_code: &str,
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    // 実装統一: 共通の全体パーサを用い、対象国コードのみ抽出
    let cc_upper = country_code.to_ascii_uppercase();
    let map = crate::parse::parse_all_country_codes(rir_texts)?;
    let (v4_vec, v6_vec) = map.get(&cc_upper).cloned().unwrap_or_default();
    let v4_set: BTreeSet<IpNet> = v4_vec.into_iter().collect();
    let v6_set: BTreeSet<IpNet> = v6_vec.into_iter().collect();
    Ok((v4_set, v6_set))
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
            debug_log(format!("No IPs found for country code: {}", upper));
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
