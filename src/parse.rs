use crate::parse_ipv4::largest_ipv4_block;
use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::net::Ipv4Addr;

/// RIR が提供するテキストデータを行ごとに解析し、
/// 指定された`country_code`に合致する IPv4/IPv6 のリストを返す。
pub fn parse_ip_lines(
    text: &str,
    country_code: &str,
) -> Result<(Vec<IpNet>, Vec<IpNet>), Box<dyn std::error::Error + Send + Sync>> {
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
                "ipv4" | "ipv6" => match parse_ip_params(&params) {
                    Ok(nets) => {
                        if ip_type == "ipv4" {
                            ipv4_list.extend(nets);
                        } else {
                            ipv6_list.extend(nets);
                        }
                    }
                    Err(e) => eprintln!("[parse_ip_lines] parse_ip_params error: {}", e),
                },
                _ => continue,
            }
        }
    }

    Ok((ipv4_list, ipv6_list))
}

/// ip_typeを判別し、対応するパース関数を呼び出す。
fn parse_ip_params(
    params: &[&str],
) -> Result<Vec<IpNet>, Box<dyn std::error::Error + Send + Sync>> {
    let ip_type = params[2];
    let start_str = params[3];
    let value_str = params[4];

    match ip_type {
        "ipv4" => parse_ipv4_range(start_str, value_str),
        "ipv6" => parse_ipv6_range(start_str, value_str),
        _ => Ok(vec![]),
    }
}

/// IPv4の範囲を細分化してCIDRブロック一覧を返す。
fn parse_ipv4_range(
    start_str: &str,
    value_str: &str,
) -> Result<Vec<IpNet>, Box<dyn std::error::Error + Send + Sync>> {
    let start_v4 = start_str.parse::<Ipv4Addr>()?;
    let width = value_str.parse::<u64>()?;
    let start_num = u32::from(start_v4);

    let end_num = start_num
        .checked_add(width as u32)
        .ok_or("IPv4 range is too large")?
        .checked_sub(1)
        .ok_or("Calculation error on IPv4 range")?;

    let mut cidrs = Vec::new();
    let mut current = start_num;

    while current <= end_num {
        let max_size = largest_ipv4_block(current, end_num);
        let net = Ipv4Net::new(Ipv4Addr::from(current), max_size)?;
        cidrs.push(IpNet::V4(net));

        let block_size = 1u32 << (32 - max_size);
        current = current.saturating_add(block_size);
    }

    Ok(cidrs)
}

/// IPv6用のCIDRをパースして返す。
fn parse_ipv6_range(
    start_str: &str,
    value_str: &str,
) -> Result<Vec<IpNet>, Box<dyn std::error::Error + Send + Sync>> {
    let cidr_str = format!("{}/{}", start_str, value_str);
    let net = cidr_str.parse::<Ipv6Net>()?;
    Ok(vec![IpNet::V6(net)])
}
