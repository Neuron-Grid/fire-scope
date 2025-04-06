use crate::ipv4_utils::ipv4_summarize_range;
use ipnet::{IpNet, Ipv6Net};
use std::cmp::{max, min};
use std::collections::BTreeSet;
use std::net::Ipv6Addr;

/// 国別IPs, AS別IPsそれぞれから得られたBTreeSet<IpNet>を受け取り、
/// 部分的に重複している範囲（サブネット）をすべてBTreeSet<IpNet>で返す。
/// 改良版: まず集約→IPv4/IPv6分割→2ポインタ方式で重複計算
pub fn find_overlaps(country_ips: &BTreeSet<IpNet>, as_ips: &BTreeSet<IpNet>) -> BTreeSet<IpNet> {
    // まず両者をaggregateしてサブネット個数を削減
    let country_agg = IpNet::aggregate(&country_ips.iter().copied().collect::<Vec<_>>());
    let as_agg = IpNet::aggregate(&as_ips.iter().copied().collect::<Vec<_>>());

    // IPv4/IPv6をそれぞれ分割
    let (mut c_v4, mut c_v6) = split_ipv4_ipv6(&country_agg);
    let (mut a_v4, mut a_v6) = split_ipv4_ipv6(&as_agg);

    // 開始アドレス順にソート
    c_v4.sort_by_key(|(start, _end)| *start);
    c_v6.sort_by_key(|(start, _end)| *start);
    a_v4.sort_by_key(|(start, _end)| *start);
    a_v6.sort_by_key(|(start, _end)| *start);

    // それぞれ2ポインタでオーバーラップを求める
    let overlap_v4 = overlap_ranges_v4(&c_v4, &a_v4);
    let overlap_v6 = overlap_ranges_v6(&c_v6, &a_v6);

    // 合体してBTreeSetに
    overlap_v4
        .into_iter()
        .chain(overlap_v6.into_iter())
        .collect()
}

/// IpNetベクタを IPv4, IPv6 それぞれ (start, end)形式に変換して返す
fn split_ipv4_ipv6(nets: &[IpNet]) -> (Vec<(u32, u32)>, Vec<(u128, u128)>) {
    let mut v4_ranges = Vec::new();
    let mut v6_ranges = Vec::new();

    for net in nets {
        match net {
            IpNet::V4(v4net) => {
                let start = u32::from(v4net.network());
                let end = u32::from(v4net.broadcast());
                v4_ranges.push((start, end));
            }
            IpNet::V6(v6net) => {
                let start = ipv6_to_u128(v6net.network());
                let end = ipv6_to_u128(v6net.broadcast());
                v6_ranges.push((start, end));
            }
        }
    }
    (v4_ranges, v6_ranges)
}

/// 2つのソート済みIPv4範囲リストを 2ポインタで走査して重複区間をCIDR単位で返す
fn overlap_ranges_v4(country: &[(u32, u32)], aslist: &[(u32, u32)]) -> Vec<IpNet> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < country.len() && j < aslist.len() {
        let (c_start, c_end) = country[i];
        let (a_start, a_end) = aslist[j];

        // 重複範囲の開始/終了
        let overlap_start = max(c_start, a_start);
        let overlap_end = min(c_end, a_end);

        if overlap_start <= overlap_end {
            // CIDR単位に分割
            let cidrs = ipv4_summarize_range(overlap_start, overlap_end);
            result.extend(cidrs);
        }

        // どちらかが先に終わるかでポインタを進める
        if c_end < a_end {
            i += 1;
        } else {
            j += 1;
        }
    }

    result
}

/// 2つのソート済みIPv6範囲リストを2ポインタで走査して重複区間をCIDR単位で返す
fn overlap_ranges_v6(country: &[(u128, u128)], aslist: &[(u128, u128)]) -> Vec<IpNet> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < country.len() && j < aslist.len() {
        let (c_start, c_end) = country[i];
        let (a_start, a_end) = aslist[j];

        let overlap_start = max(c_start, a_start);
        let overlap_end = min(c_end, a_end);

        if overlap_start <= overlap_end {
            // CIDR単位に分割
            let cidrs = ipv6_summarize_range(overlap_start, overlap_end);
            result.extend(cidrs);
        }

        if c_end < a_end {
            i += 1;
        } else {
            j += 1;
        }
    }

    result
}

/// 開始～終了アドレスを最適なIPv6 CIDRに分割
/// 従来のipv6_overlap内部ロジックを外部化
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

/// IPv6アドレスをu128に
fn ipv6_to_u128(addr: Ipv6Addr) -> u128 {
    u128::from_be_bytes(addr.octets())
}

/// IPv6の範囲を切り分けるためのブロックサイズを決定
fn largest_ipv6_block_in_overlap(current: u128, end: u128) -> u8 {
    let tz = current.trailing_zeros() as u128;
    let span = (end - current + 1).ilog2_128();
    let max_block = tz.min(span);
    (128 - max_block) as u8
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
