use super::rpki_loader::load_vrps_from_tals;
use crate::error::AppError;
use ipnet::IpNet;

/// RPKI VALIDフィルタ
pub async fn filter_valid_by_rpki(asn: u32, nets: &[IpNet]) -> Result<Vec<IpNet>, AppError> {
    let tbl_arc = load_vrps_from_tals().await?;
    let tbl = tbl_arc.read().await;

    // 当該ASNのVRPが無ければ空ベクタを返す
    let vrps = match tbl.get(&asn) {
        Some(list) => list,
        None => return Ok(vec![]),
    };

    let mut result = Vec::new();

    // 各候補プレフィックスを走査
    for net in nets {
        // 子プレフィックスのprefix長を検証
        if !validate_prefix_length(net) {
            continue;
        }

        for (parent_net, max_len) in vrps {
            // 親側prefix長も念のため検証
            if !validate_prefix_length(parent_net) {
                continue;
            }

            // RPKI規則どおり包含関係+maxLenを判定
            if is_subnet(net, parent_net) && net.prefix_len() <= *max_len {
                result.push(*net);
                // 次のnetへ
                break;
            }
        }
    }
    Ok(result)
}

/// IPv4 は /0-/32
/// IPv6 は /0-/128であることを明示的にチェック
fn validate_prefix_length(net: &IpNet) -> bool {
    match net {
        IpNet::V4(v4) => v4.prefix_len() <= 32,
        IpNet::V6(v6) => v6.prefix_len() <= 128,
    }
}

/// parentがchildを包含するか確認
/// RFC 6483/6811に準拠
fn is_subnet(child: &IpNet, parent: &IpNet) -> bool {
    match (child, parent) {
        (IpNet::V4(c), IpNet::V4(p)) => {
            p.contains(&c.network()) && c.prefix_len() >= p.prefix_len()
        }
        (IpNet::V6(c), IpNet::V6(p)) => {
            p.contains(&c.network()) && c.prefix_len() >= p.prefix_len()
        }
        // IPファミリ不一致
        _ => false,
    }
}
