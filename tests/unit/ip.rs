use super::{IpFamily, IpSets};
use ipnet::IpNet;
use std::error::Error;
use std::str::FromStr;

#[test]
fn sets_partition_merge_and_aggregate_without_changing_inputs() -> Result<(), Box<dyn Error>> {
    let first = [
        IpNet::from_str("192.0.2.0/25")?,
        IpNet::from_str("2001:db8::/32")?,
    ]
    .into_iter()
    .collect::<IpSets>();
    let second = [IpNet::from_str("192.0.2.128/25")?]
        .into_iter()
        .collect::<IpSets>();
    let first_before = first.clone();
    let second_before = second.clone();

    let aggregated = first.clone().merge(second.clone()).aggregated();

    assert_eq!(first, first_before);
    assert_eq!(second, second_before);
    assert_eq!(aggregated.ipv4().len(), 1);
    assert_eq!(
        aggregated.ipv4().iter().next().map(ToString::to_string),
        Some("192.0.2.0/24".to_owned())
    );
    assert_eq!(aggregated.ipv6(), first.ipv6());
    assert!(!aggregated.is_empty());
    assert!(IpSets::default().is_empty());
    Ok(())
}

#[test]
fn family_parsing_ignores_ascii_case() {
    assert_eq!(IpFamily::from_str("ipv4"), Ok(IpFamily::V4));
    assert_eq!(IpFamily::from_str("IPV6"), Ok(IpFamily::V6));
    assert!(IpFamily::from_str("asn").is_err());
    assert_eq!(IpFamily::V4.as_str(), "IPv4");
    assert_eq!(IpFamily::V6.as_str(), "IPv6");
}
