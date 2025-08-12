use ipnet::{IpNet, Ipv4Net};
use crate::error::AppError;
use std::net::Ipv4Addr;

pub trait ILog2Sub1 {
    fn ilog2_sub1(&self) -> u32;
}

impl ILog2Sub1 for u32 {
    fn ilog2_sub1(&self) -> u32 {
        if *self == 0 {
            0
        } else {
            31 - self.leading_zeros()
        }
    }
}

pub trait ILog2Sub1U64 {
    fn ilog2_sub1_u64(&self) -> u32;
}

impl ILog2Sub1U64 for u64 {
    fn ilog2_sub1_u64(&self) -> u32 {
        if *self == 0 {
            0
        } else {
            63 - self.leading_zeros()
        }
    }
}

/// currentから始まりendを超えない最大のIPv4 CIDRプレフィックス長(≤ 32)を返す。
pub fn largest_ipv4_block(current: u64, end: u64) -> u8 {
    debug_assert!(current <= end, "current must be <= end");

    // current(32ビット空)の末尾ゼロビットの数
    let tz: u32 = (current as u32).trailing_zeros();
    // 残りのアドレス範囲に収まるビット数
    let span: u32 = (end - current + 1).ilog2_sub1_u64();

    // ホスト部で使用可能なビット
    let max_block = tz.min(span);
    // CIDRプレフィックス長(0-32)
    (32 - max_block) as u8
}

/// IPv4の範囲[`start`, `end`]をCIDRの最小セットにまとめる。
pub fn ipv4_summarize_range(start: u64, end: u64) -> Vec<IpNet> {
    let mut cidrs = Vec::<IpNet>::new();
    let mut current = start;

    while current <= end {
        let max_size = largest_ipv4_block(current, end);

        // IPv4Netは32ビットアドレスのみをサポートする
        if current > u32::MAX as u64 {
            // 範囲外のセクションを無視する
            break;
        }

        if let Ok(net) = Ipv4Net::new(Ipv4Addr::from(current as u32), max_size) {
            cidrs.push(IpNet::V4(net));
            let block_size: u64 = 1u64 << (32 - max_size);
            current = current.saturating_add(block_size);
        } else {
            // フェイルセーフ
            break;
        }
    }

    cidrs
}

/// RIR拡張フォーマットのIPv4行（start, value）をCIDR列へ展開
pub fn parse_ipv4_range_to_cidrs(start_str: &str, value_str: &str) -> Result<Vec<IpNet>, AppError> {
    let start_addr = start_str.parse::<Ipv4Addr>()?;
    let width_u64 = value_str.parse::<u64>()?;

    if width_u64 == 0 {
        return Err(AppError::ParseError("IPv4 width must be > 0".into()));
    }

    let start_num = u32::from(start_addr) as u64;
    let end_num_u64 = start_num
        .checked_add(width_u64)
        .and_then(|v| v.checked_sub(1))
        .ok_or_else(|| AppError::ParseError("IPv4 range is too large".into()))?;

    if end_num_u64 > u32::MAX as u64 {
        return Err(AppError::ParseError(
            "IPv4 range exceeds 32‑bit boundary".into(),
        ));
    }

    Ok(ipv4_summarize_range(start_num, end_num_u64))
}
