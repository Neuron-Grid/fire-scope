use super::find_overlaps;
use crate::ip::IpSets;
use ipnet::IpNet;
use std::error::Error;
use std::str::FromStr;

#[test]
fn finds_ipv4_and_ipv6_intersections() -> Result<(), Box<dyn Error>> {
    let country = [
        IpNet::from_str("10.0.0.0/24")?,
        IpNet::from_str("10.0.1.0/24")?,
        IpNet::from_str("2001:db8::/32")?,
    ]
    .into_iter()
    .collect::<IpSets>();
    let asn = [
        IpNet::from_str("10.0.0.128/25")?,
        IpNet::from_str("2001:db8:8000::/33")?,
    ]
    .into_iter()
    .collect::<IpSets>();

    let overlaps = find_overlaps(&country, &asn)?;
    assert_eq!(
        overlaps
            .ipv4()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["10.0.0.128/25"]
    );
    assert_eq!(
        overlaps
            .ipv6()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["2001:db8:8000::/33"]
    );
    Ok(())
}

#[test]
fn preserves_full_and_upper_half_ipv6_ranges() -> Result<(), Box<dyn Error>> {
    let full = [IpNet::from_str("::/0")?].into_iter().collect::<IpSets>();
    let upper = [IpNet::from_str("8000::/1")?]
        .into_iter()
        .collect::<IpSets>();

    let full_overlap = find_overlaps(&full, &full)?;
    let upper_overlap = find_overlaps(&full, &upper)?;
    assert_eq!(
        full_overlap
            .ipv6()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["::/0"]
    );
    assert_eq!(
        upper_overlap
            .ipv6()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["8000::/1"]
    );
    Ok(())
}
