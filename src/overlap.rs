use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::cmp::{max, min};
use std::collections::BTreeSet;
use std::net::{Ipv4Addr, Ipv6Addr};

/// 国別IPs, AS別IPs それぞれから得られたBTreeSet<IpNet>を受け取り、
/// 部分的に重複している範囲（サブネット）をすべてBTreeSet<IpNet>で返す。
pub fn find_overlaps(country_ips: &BTreeSet<IpNet>, as_ips: &BTreeSet<IpNet>) -> BTreeSet<IpNet> {
    let mut result = BTreeSet::new();

    for cnet in country_ips {
        for anet in as_ips {
            let overlap_cidrs = ipnet_overlap(cnet, anet);
            result.extend(overlap_cidrs);
        }
    }
    result
}

/// 2つのIpNetの重複範囲をCIDRのリストで返す。
/// 部分的にでも被っていればOK
fn ipnet_overlap(a: &IpNet, b: &IpNet) -> Vec<IpNet> {
    match (a, b) {
        (IpNet::V4(a4), IpNet::V4(b4)) => ipv4_overlap(a4, b4),
        (IpNet::V6(a6), IpNet::V6(b6)) => ipv6_overlap(a6, b6),
        // IPv4 と IPv6 は重複しない
        _ => Vec::new(),
    }
}

/// IPv4同士の重複範囲を求めてCIDR列として返す
fn ipv4_overlap(a: &Ipv4Net, b: &Ipv4Net) -> Vec<IpNet> {
    let a_start = u32::from(a.network());
    let a_end = u32::from(a.broadcast());
    let b_start = u32::from(b.network());
    let b_end = u32::from(b.broadcast());

    let overlap_start = max(a_start, b_start);
    let overlap_end = min(a_end, b_end);
    if overlap_start > overlap_end {
        return Vec::new();
    }

    ipv4_summarize_range(overlap_start, overlap_end)
}

/// IPv4用
/// 開始～終了アドレスを最適なCIDRに分割
fn ipv4_summarize_range(start: u32, end: u32) -> Vec<IpNet> {
    let mut cidrs = Vec::new();
    let mut current = start;

    while current <= end {
        let max_size = largest_ipv4_block_in_overlap(current, end);
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

/// IPv4用
/// 重複計算用の最大ブロックサイズ判定
fn largest_ipv4_block_in_overlap(current: u32, end: u32) -> u8 {
    let tz = current.trailing_zeros();
    // Rust 1.67+ で使える標準ライブラリの ilog2() を利用
    let span = (end - current + 1).ilog2();
    let max_block = tz.min(span);
    (32 - max_block) as u8
}

/// IPv6同士の重複範囲を求めてCIDR列として返す
fn ipv6_overlap(a: &Ipv6Net, b: &Ipv6Net) -> Vec<IpNet> {
    let a_start = ipv6_to_u128(a.network());
    let a_end = ipv6_to_u128(a.broadcast());
    let b_start = ipv6_to_u128(b.network());
    let b_end = ipv6_to_u128(b.broadcast());

    let overlap_start = max(a_start, b_start);
    let overlap_end = min(a_end, b_end);
    if overlap_start > overlap_end {
        return Vec::new();
    }

    ipv6_summarize_range(overlap_start, overlap_end)
}

/// IPv6用
/// 開始～終了アドレスを最適なCIDRに分割
fn ipv6_summarize_range(start: u128, end: u128) -> Vec<IpNet> {
    let mut cidrs = Vec::new();
    let mut current = start;

    while current <= end {
        let max_size = largest_ipv6_block_in_overlap(current, end);
        if let Ok(net) = Ipv6Net::new(Ipv6Addr::from(current), max_size) {
            cidrs.push(IpNet::V6(net));
            let block_size = 1u128 << (128 - max_size);
            current = current.saturating_add(block_size);
        } else {
            break;
        }
    }

    cidrs
}

/// IPv6用
/// 重複計算用の最大ブロックサイズ判定
fn largest_ipv6_block_in_overlap(current: u128, end: u128) -> u8 {
    let tz = current.trailing_zeros() as u128;
    // u128 用はまだ標準で ilog2() がないため、自前トレイトを使用
    let span = (end - current + 1).ilog2_128();
    let max_block = tz.min(span);
    (128 - max_block) as u8
}

/// ヘルパー関数
/// Ipv6Addrをu128に変換
fn ipv6_to_u128(addr: Ipv6Addr) -> u128 {
    u128::from_be_bytes(addr.octets())
}

/// u128用のilog2相当
trait ILog2U128 {
    fn ilog2_128(self) -> u128;
}

impl ILog2U128 for u128 {
    fn ilog2_128(self) -> u128 {
        if self == 0 {
            0
        } else {
            127 - self.leading_zeros() as u128
        }
    }
}
