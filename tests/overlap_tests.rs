use fire_scope::overlap::find_overlaps;
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::str::FromStr;

fn ipnet(s: &str) -> IpNet {
    match IpNet::from_str(s) {
        Ok(n) => n,
        Err(e) => panic!("failed to parse {s}: {e}"),
    }
}

#[test]
fn find_overlaps_ipv4_and_ipv6_basic() {
    let mut country = BTreeSet::new();
    country.insert(ipnet("10.0.0.0/24"));
    country.insert(ipnet("10.0.1.0/24"));
    country.insert(ipnet("2001:db8::/32"));

    let mut aslist = BTreeSet::new();
    aslist.insert(ipnet("10.0.0.128/25"));
    aslist.insert(ipnet("2001:db8:8000::/33"));

    let overlaps = find_overlaps(&country, &aslist);
    let got: Vec<String> = overlaps.into_iter().map(|n| n.to_string()).collect();

    // 順序はBTreeSet依存だが、要素集合が等しいことを確認
    assert!(got.contains(&"10.0.0.128/25".to_string()));
    assert!(got.contains(&"2001:db8:8000::/33".to_string()));
}

#[test]
fn find_overlaps_ipv6_full_range_does_not_panic() {
    let mut country = BTreeSet::new();
    country.insert(ipnet("::/0"));

    let mut aslist = BTreeSet::new();
    aslist.insert(ipnet("::/0"));

    let overlaps = find_overlaps(&country, &aslist);
    let got: Vec<String> = overlaps.into_iter().map(|n| n.to_string()).collect();

    assert_eq!(got, vec!["::/0".to_string()]);
}

#[test]
fn find_overlaps_ipv6_upper_half_is_preserved() {
    let mut country = BTreeSet::new();
    country.insert(ipnet("::/0"));

    let mut aslist = BTreeSet::new();
    aslist.insert(ipnet("8000::/1"));

    let overlaps = find_overlaps(&country, &aslist);
    let got: Vec<String> = overlaps.into_iter().map(|n| n.to_string()).collect();

    assert_eq!(got, vec!["8000::/1".to_string()]);
}
