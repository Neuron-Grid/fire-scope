use crate::ipv4_utils::ipv4_summarize_range;
use ipnet::{IpNet, Ipv6Net};
use std::cmp::{max, min};
use std::collections::BTreeSet;
use std::net::Ipv6Addr;

pub fn find_overlaps(country_ips: &BTreeSet<IpNet>, as_ips: &BTreeSet<IpNet>) -> BTreeSet<IpNet> {
    // aggregateでプレフィックス数を削減
    let country_agg = IpNet::aggregate(&country_ips.iter().copied().collect::<Vec<_>>());
    let as_agg = IpNet::aggregate(&as_ips.iter().copied().collect::<Vec<_>>());

    // IPv4 / IPv6を分割
    let (mut c_v4, mut c_v6) = split_ipv4_ipv6(&country_agg);
    let (mut a_v4, mut a_v6) = split_ipv4_ipv6(&as_agg);

    c_v4.sort_by_key(|(s, _)| *s);
    a_v4.sort_by_key(|(s, _)| *s);
    c_v6.sort_by_key(|(s, _)| *s);
    a_v6.sort_by_key(|(s, _)| *s);

    let o_v4 = overlap_ranges_v4(&c_v4, &a_v4);
    let o_v6 = overlap_ranges_v6(&c_v6, &a_v6);

    o_v4.into_iter().chain(o_v6).collect()
}

fn split_ipv4_ipv6(nets: &[IpNet]) -> (Vec<(u64, u64)>, Vec<(u128, u128)>) {
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();

    for net in nets {
        match net {
            IpNet::V4(n) => {
                v4.push((
                    u32::from(n.network()) as u64,
                    u32::from(n.broadcast()) as u64,
                ));
            }
            IpNet::V6(n) => {
                v6.push((ipv6_to_u128(n.network()), ipv6_to_u128(n.broadcast())));
            }
        }
    }
    (v4, v6)
}

fn overlap_ranges_v4(country: &[(u64, u64)], aslist: &[(u64, u64)]) -> Vec<IpNet> {
    let mut res = Vec::new();
    let (mut i, mut j) = (0, 0);

    while i < country.len() && j < aslist.len() {
        let (c_s, c_e) = country[i];
        let (a_s, a_e) = aslist[j];

        let s = max(c_s, a_s);
        let e = min(c_e, a_e);

        if s <= e {
            res.extend(ipv4_summarize_range(s, e));
        }
        if c_e < a_e {
            i += 1;
        } else {
            j += 1;
        }
    }
    res
}

fn overlap_ranges_v6(country: &[(u128, u128)], aslist: &[(u128, u128)]) -> Vec<IpNet> {
    let mut res = Vec::new();
    let (mut i, mut j) = (0, 0);

    while i < country.len() && j < aslist.len() {
        let (c_s, c_e) = country[i];
        let (a_s, a_e) = aslist[j];

        let s = max(c_s, a_s);
        let e = min(c_e, a_e);

        if s <= e {
            res.extend(ipv6_summarize_range(s, e));
        }
        if c_e < a_e {
            i += 1;
        } else {
            j += 1;
        }
    }
    res
}

fn ipv6_summarize_range(start: u128, end: u128) -> Vec<IpNet> {
    let mut cidrs = Vec::new();
    let mut cur = start;

    while cur <= end {
        let max = largest_ipv6_block_in_overlap(cur, end);
        if let Ok(net) = Ipv6Net::new(Ipv6Addr::from(cur), max) {
            cidrs.push(IpNet::V6(net));
            let step = 1u128 << (128 - max);
            cur = cur.saturating_add(step);
        } else {
            break;
        }
    }

    cidrs
}

fn ipv6_to_u128(addr: Ipv6Addr) -> u128 {
    u128::from_be_bytes(addr.octets())
}

fn largest_ipv6_block_in_overlap(current: u128, end: u128) -> u8 {
    let tz: u32 = current.trailing_zeros();
    let span: u32 = ((end - current + 1).ilog2_128()) as u32;
    let max: u32 = tz.min(span);
    (128 - max) as u8
}

trait ILog2U128 {
    fn ilog2_128(self) -> u32;
}
impl ILog2U128 for u128 {
    fn ilog2_128(self) -> u32 {
        if self == 0 {
            0
        } else {
            127 - self.leading_zeros()
        }
    }
}
