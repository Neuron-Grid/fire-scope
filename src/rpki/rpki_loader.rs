/// RPKI VRPテーブルをRoutinatorから取得して構築する実装。
use crate::error::AppError;
use ipnet::IpNet;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{
    collections::HashMap,
    process::{Command, Stdio},
    sync::Arc,
};
use tokio::sync::RwLock;

/// ASN → [(Prefix, maxLen)] の VRP テーブル型
type VrpTable = HashMap<u32, Vec<(IpNet, u8)>>;

/// グローバル共有キャッシュ
static GLOBAL_VRP_TABLE: OnceCell<Arc<RwLock<VrpTable>>> = OnceCell::new();

/// Routinator JSON に含まれる最小限のフィールドだけ定義
#[derive(Debug, Deserialize)]
struct VrpsJson {
    roas: Vec<RoaEntry>,
}

#[derive(Debug, Deserialize)]
struct RoaEntry {
    asn: String,    // 例: "AS2497"
    prefix: String, // 例: "203.178.128.0/17"
    #[serde(rename = "maxLength")]
    max_length: u8, // 例: 24
}

/// TAL 群を読み込み VRP テーブルを返す（初回のみ実行）
pub async fn load_vrps_from_tals() -> Result<Arc<RwLock<VrpTable>>, AppError> {
    // すでに構築済みなら即return
    if let Some(tbl) = GLOBAL_VRP_TABLE.get() {
        return Ok(tbl.clone());
    }

    // Routinator を blocking スレッドで実行
    let output_res = tokio::task::spawn_blocking(|| {
        Command::new("routinator")
            .args(["vrps", "--format", "json", "--output", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output() // -> std::io::Result<Output>
    })
    .await
    .map_err(|e| AppError::Other(format!("Failed to spawn routinator: {e}")))?;

    let output = output_res?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Other(format!(
            "Routinator exited with code {}: {}",
            output.status, stderr
        )));
    }

    // JSON をパース
    let json_str = String::from_utf8(output.stdout)?;
    let vrps: VrpsJson = serde_json::from_str(&json_str)
        .map_err(|e| AppError::Other(format!("JSON parse error: {e}")))?;

    // VRPハッシュマップを構築
    let mut tbl: VrpTable = HashMap::new();

    for roa in vrps.roas {
        // "AS64500" -> 64500
        let asn_num: u32 = roa
            .asn
            .trim_start_matches("AS")
            .parse()
            .map_err(|e| AppError::Other(format!("ASN parse error: {e}")))?;

        // プレフィックスをIpNetに変換
        let net: IpNet = roa
            .prefix
            .parse()
            .map_err(|e| AppError::Other(format!("Prefix parse error: {e}")))?;

        // ROAのmaxLengthがネットワーク長より短いのは不正なのでスキップ
        if net.prefix_len() > roa.max_length {
            continue;
        }

        tbl.entry(asn_num)
            .or_insert_with(Vec::new)
            .push((net, roa.max_length));
    }

    // OnceCell へ格納して返却
    let arc = Arc::new(RwLock::new(tbl));
    // 失敗しても無視
    let _ = GLOBAL_VRP_TABLE.set(arc.clone());
    Ok(arc)
}
