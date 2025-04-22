use super::rpki_loader::load_vrps_from_tals;
use crate::error::AppError;
use ipnet::IpNet;
// RPKIバリデーションのシンプル実装。
// 現状ではrpki_loaderが返すVRPテーブルを参照して(ASN, Prefix)が有効かどうかだけ判定する。

/// 与えられた(asn, net)がVRPでVALIDならtrue
pub async fn is_valid(asn: u32, net: &IpNet) -> Result<bool, AppError> {
    // 共有VRPテーブルを取得
    let tbl_arc = load_vrps_from_tals().await?;
    let tbl = tbl_arc.read().await;

    // 対応ASNが存在する場合のみ判定
    if let Some(entries) = tbl.get(&asn) {
        Ok(entries.iter().any(|(vrp_net, max_len)| {
            vrp_net.contains(&net.network()) && net.prefix_len() <= *max_len as u8
        }))
    } else {
        Ok(false)
    }
}
