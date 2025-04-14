use crate::common::{IpFamily, OutputFormat};
use crate::error::AppError;
use crate::output::write_as_ip_list_to_file;
use ipnet::IpNet;
use reqwest::Client;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::Arc,
};
use tokio::process::Command;
use tokio::sync::Semaphore;

/// rpki-client -j の出力を想定した構造体
#[derive(Debug, Deserialize)]
struct RpkiClientJson {
    roas: Vec<RpkiRoaItem>,
}

#[derive(Debug, Deserialize)]
struct RpkiRoaItem {
    asn: String,
    prefix: String,
    #[serde(rename = "maxLength")]
    max_length: u8,
}

/// RPKIのROA情報: (prefix, asn) => max_length
type RoaMap = BTreeMap<(String, String), u8>;

// rpki-client コマンドを叩いて最新のROAデータを更新する
pub async fn update_rpki_data() -> Result<(), AppError> {
    // rpki-client -j /var/lib/rpki-client
    let status = Command::new("rpki-client")
        .arg("-j")
        .arg("/var/lib/rpki-client")
        .status()
        // Ioエラーで落ちる場合かも
        .await?;

    if !status.success() {
        return Err(AppError::Other("rpki-client command failed".into()));
    }
    Ok(())
}

// 生成された JSON をパースして BTreeMap<(prefix, asn), maxLen> へ変換
pub fn load_rpki_roa<P: AsRef<Path>>(path: P) -> Result<RoaMap, AppError> {
    let data = fs::read_to_string(path)?;
    let rpki_json: RpkiClientJson = serde_json::from_str(&data)
        .map_err(|e| AppError::ParseError(format!("Failed to parse RPKI-client JSON: {}", e)))?;

    let mut roa_map = BTreeMap::new();
    for roa in rpki_json.roas {
        // 例: roa.asn="AS13335" → "13335"
        let as_str = roa.asn.trim_start_matches("AS").to_string();
        let key = (roa.prefix.clone(), as_str);
        roa_map.insert(key, roa.max_length);
    }
    Ok(roa_map)
}

// 単純なRPKI検証
// prefix完全一致 + prefix_len <= maxLen
fn validate_with_rpki(
    prefix: &IpNet,
    as_number: &str,
    roa_map: &BTreeMap<(String, String), u8>,
) -> bool {
    let as_clean = as_number.trim_start_matches("AS");
    let prefix_str = prefix.to_string();

    if let Some(&max_len) = roa_map.get(&(prefix_str, as_clean.to_string())) {
        if prefix.prefix_len() <= max_len {
            return true;
        }
    }
    false
}

// RIPEstat Announced Prefixes API レスポンス定義
#[derive(Deserialize, Debug)]
struct RipeStatAnnouncedPrefixes {
    data: AnnouncedPrefixesData,
}

#[derive(Deserialize, Debug)]
struct AnnouncedPrefixesData {
    prefixes: Vec<AnnouncedPrefix>,
}

#[derive(Deserialize, Debug)]
struct AnnouncedPrefix {
    prefix: String,
}

// BGPデータ取得
// RPKI検証 (Option<Arc<RoaMap>>)
pub async fn get_ips_for_as_once(
    client: &Client,
    as_number: &str,
    roa_map: Option<Arc<RoaMap>>,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let url = format!(
        "https://stat.ripe.net/data/announced-prefixes/data.json?resource={}",
        as_number
    );

    let resp = client.get(&url).send().await?;
    let body = resp.text().await?;
    let parsed: RipeStatAnnouncedPrefixes = serde_json::from_str(&body)
        .map_err(|e| AppError::ParseError(format!("Failed to parse RIPEstat JSON: {}", e)))?;

    let mut v4s = BTreeSet::new();
    let mut v6s = BTreeSet::new();

    for pfx in parsed.data.prefixes {
        if let Ok(ipnet) = pfx.prefix.parse::<IpNet>() {
            // RPKI 検証
            if let Some(ref roa) = roa_map {
                if !validate_with_rpki(&ipnet, as_number, roa) {
                    // 無効と判定されたらスキップ
                    continue;
                }
            }
            match ipnet {
                IpNet::V4(_) => {
                    v4s.insert(ipnet);
                }
                IpNet::V6(_) => {
                    v6s.insert(ipnet);
                }
            }
        }
    }

    // aggregate
    let aggregated_v4 = IpNet::aggregate(&v4s.iter().copied().collect::<Vec<_>>());
    let aggregated_v6 = IpNet::aggregate(&v6s.iter().copied().collect::<Vec<_>>());

    Ok((
        aggregated_v4.into_iter().collect(),
        aggregated_v6.into_iter().collect(),
    ))
}

/// 複数ASを並行処理
/// 毎回 rpki-client で最新化 → JSON読み込み → get_ips_for_as_once()
pub async fn process_as_numbers(
    client: &Client,
    as_numbers: &[String],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    // コマンド実行ごとに最新のROAを取得
    update_rpki_data().await?;

    // JSONをロード
    // 失敗したら空Mapを返して検証スキップ
    let rpki_path = "/var/lib/rpki-client/out.json";
    let roa_map = match load_rpki_roa(rpki_path) {
        // 成功: Arc<BTreeMap<...>>
        Ok(m) => Arc::new(m),
        Err(e) => {
            eprintln!(
                "Warning: RPKI data load failed: {}. Skipping RPKI checks...",
                e
            );
            // 空を返して全Accept
            Arc::new(BTreeMap::new())
        }
    };

    let max_concurrent = 5;
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = Vec::new();

    for as_number in as_numbers {
        let as_number_clone = as_number.clone();
        let mode_clone = mode.to_string();
        let format_clone = output_format;
        let sem_clone = semaphore.clone();
        let client_clone = client.clone();
        let roa_clone = roa_map.clone(); // Arcクローン

        let handle = tokio::spawn(async move {
            let _permit = sem_clone.acquire_owned().await?;
            match get_ips_for_as_once(&client_clone, &as_number_clone, Some(roa_clone)).await {
                Ok((v4set, v6set)) => {
                    if v4set.is_empty() {
                        println!("[asn] No (valid) IPv4 routes found for {}", as_number_clone);
                    } else {
                        write_as_ip_list_to_file(
                            &as_number_clone,
                            IpFamily::V4,
                            &v4set,
                            &mode_clone,
                            format_clone,
                        )
                        .await?;
                    }
                    if v6set.is_empty() {
                        println!("[asn] No (valid) IPv6 routes found for {}", as_number_clone);
                    } else {
                        write_as_ip_list_to_file(
                            &as_number_clone,
                            IpFamily::V6,
                            &v6set,
                            &mode_clone,
                            format_clone,
                        )
                        .await?;
                    }
                }
                Err(e) => eprintln!("[asn] Error processing {}: {}", as_number_clone, e),
            };
            Ok::<(), AppError>(())
        });
        handles.push(handle);
    }

    // タスク完了を待つ
    for h in handles {
        h.await??;
    }

    Ok(())
}
