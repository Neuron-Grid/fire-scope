use fire_scope::ipv4_utils::{ipv4_summarize_range, largest_ipv4_block, parse_ipv4_range_to_cidrs};
use fire_scope::error::AppError;
use ipnet::Ipv4Net;
use std::net::Ipv4Addr;

#[test]
fn largest_block_basic_cases() {
    // 0.0.0.0..=0.0.0.255 → /24
    let p = largest_ipv4_block(0, 255);
    assert_eq!(p, 24);

    // 0.0.0.0..=0.0.1.255 → /23
    let p = largest_ipv4_block(0, 511);
    assert_eq!(p, 23);

    // 単一アドレス → /32
    let p = largest_ipv4_block(1, 1);
    assert_eq!(p, 32);
}

#[test]
fn summarizes_ipv4_range_minimal_sets() {
    // 0..=255 → 0.0.0.0/24
    let cidrs = ipv4_summarize_range(0, 255);
    assert_eq!(cidrs.len(), 1);
    match &cidrs[0] {
        ipnet::IpNet::V4(n) => assert_eq!(n, &Ipv4Net::new(Ipv4Addr::new(0, 0, 0, 0), 24).unwrap()),
        _ => panic!("unexpected v6"),
    }

    // 1..=3 → 0.0.0.1/32, 0.0.0.2/31
    let cidrs = ipv4_summarize_range(1, 3);
    assert_eq!(cidrs.len(), 2);
    let got: Vec<String> = cidrs.iter().map(|n| n.to_string()).collect();
    assert_eq!(got, vec!["0.0.0.1/32", "0.0.0.2/31"]);
}

#[test]
fn parses_ipv4_range_to_cidrs() -> Result<(), AppError> {
    // 幅1 → /32
    let v = parse_ipv4_range_to_cidrs("1.2.3.4", "1")?;
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].to_string(), "1.2.3.4/32");

    // 幅256 aligned → /24
    let v = parse_ipv4_range_to_cidrs("1.2.3.0", "256")?;
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].to_string(), "1.2.3.0/24");

    // 幅0 → エラー
    let e = parse_ipv4_range_to_cidrs("1.2.3.4", "0");
    assert!(e.is_err());

    // 32bit境界超過 → エラー
    let e = parse_ipv4_range_to_cidrs("255.255.255.255", "2");
    assert!(e.is_err());
    Ok(())
}

