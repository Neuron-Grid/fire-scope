//! RPKI VRP テーブルを Routinator 0.13 ライブラリ API
//! (`operation::vrps::VrpsBuilder`) だけで構築し、OnceCell にキャッシュする。
//!
//! Cargo.toml には
//!     routinator = "=0.13.2"
//! を指定しておくこと。

use crate::error::AppError;
use ipnet::IpNet;
use once_cell::sync::OnceCell;
use routinator::{
    config::Config,
    operation::vrps::VrpsBuilder, // ← 0.13 では公開されている
    rpki::repository::x509::Time,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// ASN → [(Prefix, max_len)]
type VrpTable = HashMap<u32, Vec<(IpNet, u8)>>;

/// グローバル共有キャッシュ
static GLOBAL_VRP_TABLE: OnceCell<Arc<RwLock<VrpTable>>> = OnceCell::new();

/// VRP テーブル取得（初回のみビルド）
pub async fn load_vrps_from_tals() -> Result<Arc<RwLock<VrpTable>>, AppError> {
    // すでにロード済みなら即 return
    if let Some(tbl) = GLOBAL_VRP_TABLE.get() {
        return Ok(tbl.clone());
    }

    // Routinator の VRP 生成は CPU バウンドなので blocking にオフロード
    let table = tokio::task::spawn_blocking(|| -> Result<VrpTable, AppError> {
        //----------------------------------------------------------
        // 1. Config を組み立て（デフォルト設定で十分）
        //----------------------------------------------------------
        let cfg = Config::default();

        //----------------------------------------------------------
        // 2. VrpsBuilder で検証 & VRP 一覧取得
        //----------------------------------------------------------
        let now = Time::now();
        let (payload, _stats) = VrpsBuilder::default()
            .validate_at(now)
            .build(&cfg) // (Payload, Stats)
            .map_err(|e| AppError::Other(format!("Routinator validation error: {e}")))?;

        //----------------------------------------------------------
        // 3. Payload → HashMap<u32, Vec<(IpNet,u8)>> へ変換
        //----------------------------------------------------------
        let mut tbl: VrpTable = HashMap::new();

        for rec in payload.iter() {
            let net: IpNet = rec
                .prefix()
                .to_string()
                .parse()
                .map_err(|e| AppError::Other(format!("Prefix parse error: {e}")))?;
            tbl.entry(rec.asn().into_u32())
                .or_default()
                .push((net, rec.max_len()));
        }

        Ok(tbl)
    })
    .await
    .map_err(|e| AppError::Other(format!("Blocking task panicked: {e}")))??;

    let arc = Arc::new(RwLock::new(table));
    // OnceCell に格納
    let _ = GLOBAL_VRP_TABLE.set(arc.clone());
    Ok(arc)
}
