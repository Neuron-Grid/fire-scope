use super::{CountryMap, select_country_ips};
use crate::ip::IpSets;
use ipnet::IpNet;
use std::str::FromStr;

fn ipnet(cidr: &str) -> Option<IpNet> {
    IpNet::from_str(cidr).ok()
}

#[test]
fn selects_and_merges_countries_without_changing_the_map() {
    let jp = [ipnet("192.0.2.0/24")]
        .into_iter()
        .flatten()
        .collect::<IpSets>();
    let us = [ipnet("2001:db8::/32")]
        .into_iter()
        .flatten()
        .collect::<IpSets>();
    let country_map = CountryMap::from([("JP".to_string(), jp), ("US".to_string(), us)]);
    let original = country_map.clone();
    let country_codes = ["JP".into(), "US".into(), "ZZ".into()];

    let selection = select_country_ips(&country_map, &country_codes);
    let merged = selection.merged_ips();

    assert_eq!(country_map, original);
    assert_eq!(merged.ipv4().len(), 1);
    assert_eq!(merged.ipv6().len(), 1);
    assert_eq!(selection.missing_codes, ["ZZ"]);
}
