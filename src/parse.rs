use crate::error::AppError;
use ipnet::{IpNet, Ipv6Net};
use std::collections::{BTreeSet, HashMap};
use rayon::prelude::*;
use rayon::join;

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
    // RIRファイル単位のパースをrayonで並列化し、結果を順次マージ
    let partials: Vec<Result<HashMap<String, (BTreeSet<IpNet>, BTreeSet<IpNet>)>, AppError>> =
        rir_texts
            .par_iter()
            .map(|text| parse_one_rir_text_to_sets(text))
            .collect();

    let mut country_sets: HashMap<String, (BTreeSet<IpNet>, BTreeSet<IpNet>)> = HashMap::new();
    for res in partials {
        let map = res?;
        for (cc, (v4s, v6s)) in map.into_iter() {
            let entry = country_sets
                .entry(cc)
                .or_insert((BTreeSet::new(), BTreeSet::new()));
            entry.0.extend(v4s);
            entry.1.extend(v6s);
        }
    }

    // 集約してVecへ変換（最小CIDR化）— 国ごとに並列実行
    let aggregated: Vec<(String, (Vec<IpNet>, Vec<IpNet>))> = country_sets
        .into_iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|(cc, (v4set, v6set))| {
            let v4_vec = v4set.iter().copied().collect::<Vec<_>>();
            let v6_vec = v6set.iter().copied().collect::<Vec<_>>();

            let (agg_v4, agg_v6) = join(
                || IpNet::aggregate(&v4_vec),
                || IpNet::aggregate(&v6_vec),
            );

            (cc, (agg_v4, agg_v6))
        })
        .collect();

    let mut country_map: HashMap<String, (Vec<IpNet>, Vec<IpNet>)> = HashMap::new();
    for (cc, pair) in aggregated {
        country_map.insert(cc, pair);
    }
    Ok(country_map)
}

// 単一RIRテキストをパースし、国コード→(v4セット, v6セット)の部分結果を返す
fn parse_one_rir_text_to_sets(
    text: &str,
) -> Result<HashMap<String, (BTreeSet<IpNet>, BTreeSet<IpNet>)>, AppError> {
    let mut country_sets: HashMap<String, (BTreeSet<IpNet>, BTreeSet<IpNet>)> = HashMap::new();

    for line in text.lines() {
        if line.starts_with('#') || line.contains('*') || line.contains("reserved") {
            continue;
        }
        let params: Vec<&str> = line.split('|').collect();
        if params.len() < 7 {
            continue;
        }

        let status = params[6].to_ascii_lowercase();
        if status != "allocated" && status != "assigned" {
            continue;
        }

        let country_code = params[1].to_uppercase();
        match params[2] {
            "ipv4" | "ipv6" => {
                let nets = parse_ip_params(&params)?;
                let entry = country_sets
                    .entry(country_code)
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

    Ok(country_sets)
}
