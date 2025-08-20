use fire_scope::parse::{parse_all_country_codes, parse_ip_lines};

fn lines_sample() -> String {
    let mut s = String::new();
    s.push_str("# comment\n");
    s.push_str("apnic|JP|ipv4|1.2.3.0|256|20200101|allocated\n");
    s.push_str("apnic|JP|ipv4|1.2.4.0|256|20200101|available\n"); // skip by status
    s.push_str("apnic|jp|ipv6|2001:db8::|32|20200101|ASSIGNED\n");
    s.push_str("ripe|US|ipv4|203.0.113.0|256|20200101|allocated\n");
    s.push_str("apnic|JP|asn|12345|1|20200101|allocated\n"); // not ip
    s
}

#[test]
fn parses_ip_lines_ipv4_and_ipv6_allocated_only() {
    let text = lines_sample();
    let res = parse_ip_lines(&text, "JP");
    assert!(res.is_ok());
    let (v4, v6) = res.unwrap_or_else(|e| panic!("unexpected error: {e}"));
    let v4s: Vec<String> = v4.iter().map(|n| n.to_string()).collect();
    let v6s: Vec<String> = v6.iter().map(|n| n.to_string()).collect();
    assert!(v4s.contains(&"1.2.3.0/24".to_string()));
    assert!(v6s.contains(&"2001:db8::/32".to_string()));
    assert!(v4s.iter().all(|s| s != "1.2.4.0/24")); // available is skipped
}

#[test]
fn parse_all_country_codes_aggregates_per_country() {
    // 2分割の/25を与え、aggregateで/24になることを確認
    let rir1 = "apnic|JP|ipv4|10.0.0.0|128|20200101|allocated\n";
    let rir2 = "apnic|JP|ipv4|10.0.0.128|128|20200101|allocated\n";
    let vec = vec![rir1.to_string(), rir2.to_string()];
    let map = parse_all_country_codes(&vec).unwrap_or_else(|e| panic!("parse err: {e}"));
    let (v4, v6) = map.get("JP").cloned().unwrap_or_default();
    assert!(v6.is_empty());
    let v4s: Vec<String> = v4.iter().map(|n| n.to_string()).collect();
    assert_eq!(v4s, vec!["10.0.0.0/24".to_string()]);
}
