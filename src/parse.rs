use crate::error::AppError;
use ipnet::{IpNet, Ipv6Net};
use std::collections::{BTreeSet, HashMap};

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
        if params.len() < 7 {
            continue;
        }

        // status 列を厳密に判定（allocated / assigned のみ採用）
        let status = params[6].to_ascii_lowercase();
        if status != "allocated" && status != "assigned" {
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
        "ipv4" => crate::ipv4_utils::parse_ipv4_range_to_cidrs(params[3], params[4]),
        "ipv6" => parse_ipv6_range(params[3], params[4]),
        _ => Ok(vec![]),
    }
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
    // 重複排除のため、まずは集合で保持
    let mut country_sets: HashMap<String, (BTreeSet<IpNet>, BTreeSet<IpNet>)> = HashMap::new();

    for text in rir_texts {
        for line in text.lines() {
            if line.starts_with('#') || line.contains('*') || line.contains("reserved") {
                continue;
            }
            let params: Vec<&str> = line.split('|').collect();
            if params.len() < 7 {
                continue;
            }

            // status 列を厳密に判定（allocated / assigned のみ採用）
            let status = params[6].to_ascii_lowercase();
            if status != "allocated" && status != "assigned" {
                continue;
            }

            let country_code = params[1].to_uppercase();
            match params[2] {
                "ipv4" | "ipv6" => {
                    let nets = parse_ip_params(&params)?;
                    let entry = country_sets
                        .entry(country_code.clone())
                        .or_insert((BTreeSet::new(), BTreeSet::new()));
                    if params[2] == "ipv4" {
                        for n in nets { entry.0.insert(n); }
                    } else {
                        for n in nets { entry.1.insert(n); }
                    }
                }
                _ => {}
            }
        }
    }
    // 集約してVecへ変換
    let mut country_map: HashMap<String, (Vec<IpNet>, Vec<IpNet>)> = HashMap::new();
    for (cc, (v4set, v6set)) in country_sets.into_iter() {
        let agg_v4 = IpNet::aggregate(&v4set.iter().copied().collect::<Vec<_>>());
        let agg_v6 = IpNet::aggregate(&v6set.iter().copied().collect::<Vec<_>>());
        country_map.insert(cc, (agg_v4, agg_v6));
    }
    Ok(country_map)
}
