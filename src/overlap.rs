use crate::error::AppError;
use crate::ip::IpSets;
use crate::ipv4_utils::ipv4_summarize_range;
use ipnet::{IpNet, Ipv6Net};
use std::cmp::{max, min};
use std::net::Ipv6Addr;

type V4Range = (u32, u32);
type V6Range = (u128, u128);

pub(crate) fn find_overlaps(country: &IpSets, asn: &IpSets) -> Result<IpSets, AppError> {
    let country = country.clone().aggregated();
    let asn = asn.clone().aggregated();

    let mut country_v4 = ipv4_ranges(&country);
    let mut asn_v4 = ipv4_ranges(&asn);
    let mut country_v6 = ipv6_ranges(&country);
    let mut asn_v6 = ipv6_ranges(&asn);
    country_v4.sort_unstable_by_key(|(start, _)| *start);
    asn_v4.sort_unstable_by_key(|(start, _)| *start);
    country_v6.sort_unstable_by_key(|(start, _)| *start);
    asn_v6.sort_unstable_by_key(|(start, _)| *start);

    let ipv4 = overlap_ranges(&country_v4, &asn_v4, ipv4_summarize_range)?;
    let ipv6 = overlap_ranges(&country_v6, &asn_v6, ipv6_summarize_range)?;
    Ok(ipv4.into_iter().chain(ipv6).collect())
}

fn ipv4_ranges(sets: &IpSets) -> Vec<V4Range> {
    sets.ipv4()
        .iter()
        .filter_map(|net| match net {
            IpNet::V4(net) => Some((u32::from(net.network()), u32::from(net.broadcast()))),
            IpNet::V6(_) => None,
        })
        .collect()
}

fn ipv6_ranges(sets: &IpSets) -> Vec<V6Range> {
    sets.ipv6()
        .iter()
        .filter_map(|net| match net {
            IpNet::V4(_) => None,
            IpNet::V6(net) => Some((u128::from(net.network()), u128::from(net.broadcast()))),
        })
        .collect()
}

fn overlap_ranges<T, F>(
    left: &[(T, T)],
    right: &[(T, T)],
    summarize: F,
) -> Result<Vec<IpNet>, AppError>
where
    T: Copy + Ord,
    F: Fn(T, T) -> Result<Vec<IpNet>, AppError>,
{
    let mut overlaps = Vec::new();
    let mut left_index = 0_usize;
    let mut right_index = 0_usize;

    while let (Some(&(left_start, left_end)), Some(&(right_start, right_end))) =
        (left.get(left_index), right.get(right_index))
    {
        let start = max(left_start, right_start);
        let end = min(left_end, right_end);
        if start <= end {
            overlaps.extend(summarize(start, end)?);
        }

        if left_end < right_end {
            left_index = left_index.saturating_add(1);
        } else {
            right_index = right_index.saturating_add(1);
        }
    }

    Ok(overlaps)
}

fn ipv6_summarize_range(start: u128, end: u128) -> Result<Vec<IpNet>, AppError> {
    if start > end {
        return Err(AppError::ParseError(
            "IPv6 range start must not exceed its end".to_owned(),
        ));
    }

    let mut cidrs = Vec::new();
    let mut current = start;

    loop {
        let prefix = largest_ipv6_block(current, end)?;
        let network = Ipv6Net::new(Ipv6Addr::from(current), prefix)
            .map_err(|error| AppError::ParseError(format!("Invalid IPv6 network: {error}")))?;
        cidrs.push(IpNet::V6(network));

        if prefix == 0 {
            break;
        }
        let host_bits = 128_u8
            .checked_sub(prefix)
            .ok_or_else(|| AppError::ParseError("Invalid IPv6 prefix length".to_owned()))?;
        let block_size = 1_u128
            .checked_shl(u32::from(host_bits))
            .ok_or_else(|| AppError::ParseError("IPv6 block size overflow".to_owned()))?;
        let Some(next) = current.checked_add(block_size) else {
            break;
        };
        if next > end {
            break;
        }
        current = next;
    }

    Ok(cidrs)
}

fn largest_ipv6_block(current: u128, end: u128) -> Result<u8, AppError> {
    if current > end {
        return Err(AppError::ParseError(
            "IPv6 range start must not exceed its end".to_owned(),
        ));
    }

    let span_bits = if current == 0 && end == u128::MAX {
        128
    } else {
        end.checked_sub(current)
            .and_then(|difference| difference.checked_add(1))
            .ok_or_else(|| AppError::ParseError("IPv6 range length overflow".to_owned()))?
            .ilog2()
    };
    let prefix = 128_u32
        .checked_sub(current.trailing_zeros().min(span_bits))
        .ok_or_else(|| AppError::ParseError("Invalid IPv6 prefix length".to_owned()))?;
    u8::try_from(prefix).map_err(|_| AppError::ParseError("Invalid IPv6 prefix length".to_owned()))
}

#[cfg(test)]
mod tests {
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
}
