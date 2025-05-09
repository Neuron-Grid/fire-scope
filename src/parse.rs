use crate::error::AppError;
use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::collections::HashMap;
use std::net::Ipv4Addr;

pub fn parse_ip_lines(
    text: &str,
    country_code: &str,
) -> Result<(Vec<IpNet>, Vec<IpNet>), AppError> {
    let mut ipv4_list = Vec::new();
    let mut ipv6_list = Vec::new();

    for line in text.lines() {
        if line.starts_with('#') || line.contains('*') || line.contains("reserved") {
            continue;
        }
        let params: Vec<&str> = line.split('|').collect();
        if params.len() < 5 {
            continue;
        }

        if params[1].eq_ignore_ascii_case(country_code) {
            let ip_type = params[2];
            match ip_type {
                "ipv4" | "ipv6" => {
                    let nets = parse_ip_params(&params)?;
                    if ip_type == "ipv4" {
                        ipv4_list.extend(nets);
                    } else {
                        ipv6_list.extend(nets);
                    }
                }
                _ => {}
            }
        }
    }

    Ok((ipv4_list, ipv6_list))
}

fn parse_ip_params(params: &[&str]) -> Result<Vec<IpNet>, AppError> {
    match params[2] {
        "ipv4" => parse_ipv4_range(params[3], params[4]),
        "ipv6" => parse_ipv6_range(params[3], params[4]),
        _ => Ok(vec![]),
    }
}

fn parse_ipv4_range(start_str: &str, value_str: &str) -> Result<Vec<IpNet>, AppError> {
    let start_addr = start_str.parse::<Ipv4Addr>()?;
    let width_u64 = value_str.parse::<u64>()?;

    if width_u64 == 0 {
        return Err(AppError::ParseError("IPv4 width must be > 0".into()));
    }

    let start_num = u32::from(start_addr) as u64;
    let end_num_u64 = start_num
        .checked_add(width_u64)
        .and_then(|v| v.checked_sub(1))
        .ok_or_else(|| AppError::ParseError("IPv4 range is too large".into()))?;

    if end_num_u64 > u32::MAX as u64 {
        return Err(AppError::ParseError(
            "IPv4 range exceeds 32‑bit boundary".into(),
        ));
    }

    let mut cidrs = Vec::new();
    let mut cur = start_num;

    while cur <= end_num_u64 {
        let max_size = crate::ipv4_utils::largest_ipv4_block(cur, end_num_u64);
        let net = Ipv4Net::new(Ipv4Addr::from(cur as u32), max_size)
            .map_err(|e| AppError::ParseError(format!("Ipv4Net::new error: {e}")))?;
        cidrs.push(IpNet::V4(net));

        let block_size: u64 = 1u64 << (32 - max_size);
        cur = cur.saturating_add(block_size);
    }

    Ok(cidrs)
}

fn parse_ipv6_range(start_str: &str, value_str: &str) -> Result<Vec<IpNet>, AppError> {
    let cidr = format!("{}/{}", start_str, value_str);
    let net = cidr
        .parse::<Ipv6Net>()
        .map_err(|e| AppError::ParseError(format!("Ipv6Net parse error: {e}")))?;
    Ok(vec![IpNet::V6(net)])
}

pub fn parse_all_country_codes(
    rir_texts: &[String],
) -> Result<HashMap<String, (Vec<IpNet>, Vec<IpNet>)>, AppError> {
    let mut country_map = HashMap::new();

    for text in rir_texts {
        for line in text.lines() {
            if line.starts_with('#') || line.contains('*') || line.contains("reserved") {
                continue;
            }
            let params: Vec<&str> = line.split('|').collect();
            if params.len() < 5 {
                continue;
            }

            let country_code = params[1].to_uppercase();
            match params[2] {
                "ipv4" | "ipv6" => {
                    let nets = parse_ip_params(&params)?;
                    let entry = country_map
                        .entry(country_code.clone())
                        .or_insert((Vec::new(), Vec::new()));
                    if params[2] == "ipv4" {
                        entry.0.extend(nets);
                    } else {
                        entry.1.extend(nets);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(country_map)
}
