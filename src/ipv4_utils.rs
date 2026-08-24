use crate::error::AppError;
use ipnet::{IpNet, Ipv4Net};
use std::net::Ipv4Addr;

fn largest_ipv4_block(current: u32, end: u32) -> Result<u8, AppError> {
    if current > end {
        return Err(AppError::ParseError(
            "IPv4 range start must not exceed its end".to_owned(),
        ));
    }

    let span = u64::from(end)
        .checked_sub(u64::from(current))
        .and_then(|difference| difference.checked_add(1))
        .ok_or_else(|| AppError::ParseError("IPv4 range length overflow".to_owned()))?;
    let block_bits = current.trailing_zeros().min(span.ilog2());
    let prefix = 32_u32
        .checked_sub(block_bits)
        .ok_or_else(|| AppError::ParseError("Invalid IPv4 prefix length".to_owned()))?;
    u8::try_from(prefix).map_err(|_| AppError::ParseError("Invalid IPv4 prefix length".to_owned()))
}

pub(crate) fn ipv4_summarize_range(start: u32, end: u32) -> Result<Vec<IpNet>, AppError> {
    if start > end {
        return Err(AppError::ParseError(
            "IPv4 range start must not exceed its end".to_owned(),
        ));
    }

    let mut cidrs = Vec::new();
    let mut current = start;

    loop {
        let prefix = largest_ipv4_block(current, end)?;
        let net = Ipv4Net::new(Ipv4Addr::from(current), prefix)
            .map_err(|error| AppError::ParseError(format!("Invalid IPv4 network: {error}")))?;
        cidrs.push(IpNet::V4(net));

        let host_bits = 32_u8
            .checked_sub(prefix)
            .ok_or_else(|| AppError::ParseError("Invalid IPv4 prefix length".to_owned()))?;
        let block_size = 1_u64
            .checked_shl(u32::from(host_bits))
            .ok_or_else(|| AppError::ParseError("IPv4 block size overflow".to_owned()))?;
        let next_address = u64::from(current)
            .checked_add(block_size)
            .ok_or_else(|| AppError::ParseError("IPv4 range overflow".to_owned()))?;
        if next_address > u64::from(end) {
            break;
        }
        current = u32::try_from(next_address)
            .map_err(|_| AppError::ParseError("IPv4 range exceeds 32-bit boundary".to_owned()))?;
    }

    Ok(cidrs)
}

pub(crate) fn parse_ipv4_range_to_cidrs(
    start_str: &str,
    value_str: &str,
) -> Result<Vec<IpNet>, AppError> {
    let start = u32::from(start_str.parse::<Ipv4Addr>()?);
    let width = value_str.parse::<u64>()?;
    if width == 0 {
        return Err(AppError::ParseError("IPv4 width must be > 0".to_owned()));
    }

    let end = u64::from(start)
        .checked_add(width)
        .and_then(|exclusive_end| exclusive_end.checked_sub(1))
        .ok_or_else(|| AppError::ParseError("IPv4 range is too large".to_owned()))?;
    let end = u32::try_from(end)
        .map_err(|_| AppError::ParseError("IPv4 range exceeds 32-bit boundary".to_owned()))?;

    ipv4_summarize_range(start, end)
}

#[cfg(test)]
#[path = "../tests/unit/ipv4_utils.rs"]
mod tests;
