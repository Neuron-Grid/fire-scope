use ipnet::{IpNet, Ipv4Net};
use std::net::Ipv4Addr;

/// u32用のヘルパートレイト。
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

/// IPv4 の範囲 [current, end] の中で取れる最大のCIDRブロックサイズを求める。
/// parse_ipv4.rs, overlap.rs で重複していたロジックを共通化。
pub fn largest_ipv4_block(current: u32, end: u32) -> u8 {
    let tz = current.trailing_zeros();
    let span = (end - current + 1).ilog2_sub1();
    let max_block = tz.min(span);
    (32 - max_block) as u8
}

/// IPv4の開始～終了アドレスを最適なCIDRに分割して返す。
pub fn ipv4_summarize_range(start: u32, end: u32) -> Vec<IpNet> {
    let mut cidrs = Vec::new();
    let mut current = start;

    while current <= end {
        let max_size = largest_ipv4_block(current, end);
        if let Ok(net) = Ipv4Net::new(Ipv4Addr::from(current), max_size) {
            cidrs.push(IpNet::V4(net));
            let block_size = 1u32 << (32 - max_size);
            current = current.saturating_add(block_size);
        } else {
            break;
        }
    }

    cidrs
}
