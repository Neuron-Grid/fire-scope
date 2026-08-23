use crate::common::IpVecPair;
use crate::error::AppError;
use ipnet::{IpNet, Ipv6Net};
use rayon::join;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap};

type CountrySets = HashMap<String, (BTreeSet<IpNet>, BTreeSet<IpNet>)>;

#[derive(Debug)]
struct RirAllocation<'a> {
    country_code: &'a str,
    ip_type: &'a str,
    start: &'a str,
    value: &'a str,
}

impl<'a> RirAllocation<'a> {
    fn from_line(line: &'a str) -> Option<Self> {
        if line.starts_with('#') || line.contains('*') || line.contains("reserved") {
            return None;
        }

        let mut fields = line.split('|');
        let _registry = fields.next()?;
        let country_code = fields.next()?;
        let ip_type = fields.next()?;
        let start = fields.next()?;
        let value = fields.next()?;
        let _date = fields.next()?;
        let status = fields.next()?;

        (matches!(ip_type, "ipv4" | "ipv6")
            && matches!(
                status.to_ascii_lowercase().as_str(),
                "allocated" | "assigned"
            ))
        .then_some(Self {
            country_code,
            ip_type,
            start,
            value,
        })
    }

    fn parse_nets(&self) -> Result<Vec<IpNet>, AppError> {
        match self.ip_type {
            "ipv4" => crate::ipv4_utils::parse_ipv4_range_to_cidrs(self.start, self.value),
            "ipv6" => parse_ipv6_range(self.start, self.value),
            _ => Ok(Vec::new()),
        }
    }
}

/// RIR 拡張フォーマットのテキストから指定国コードの IPv4/IPv6 を抽出する。
///
/// # Examples
///
/// ```
/// use fire_scope::parse::parse_ip_lines;
///
/// let text = "apnic|JP|ipv4|192.168.0.0|256|20200101|allocated\n";
/// let (v4, v6) = parse_ip_lines(text, "JP").unwrap();
/// assert_eq!(v4.len(), 1);
/// assert_eq!(v4[0].to_string(), "192.168.0.0/24");
/// assert!(v6.is_empty());
/// ```
pub fn parse_ip_lines(text: &str, country_code: &str) -> Result<IpVecPair, AppError> {
    text.lines()
        .filter_map(RirAllocation::from_line)
        .filter(|allocation| allocation.country_code.eq_ignore_ascii_case(country_code))
        .try_fold((Vec::new(), Vec::new()), |mut lists, allocation| {
            let target = if allocation.ip_type == "ipv4" {
                &mut lists.0
            } else {
                &mut lists.1
            };
            target.extend(allocation.parse_nets()?);
            Ok(lists)
        })
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
) -> Result<HashMap<String, IpVecPair>, AppError> {
    // RIRファイル単位のパースをrayonで並列化し、結果を順次マージ
    let partials: Vec<Result<CountrySets, AppError>> = rir_texts
        .par_iter()
        .map(|text| parse_one_rir_text_to_sets(text))
        .collect();

    let country_sets =
        partials
            .into_iter()
            .try_fold(CountrySets::new(), |mut countries, partial| {
                partial?.into_iter().for_each(|(country_code, (v4, v6))| {
                    let entry = countries.entry(country_code).or_default();
                    entry.0.extend(v4);
                    entry.1.extend(v6);
                });
                Ok::<_, AppError>(countries)
            })?;

    // 集約してVecへ変換（最小CIDR化）— 国ごとに並列実行
    let aggregated: Vec<(String, IpVecPair)> = country_sets
        .into_iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|(cc, (v4set, v6set))| {
            let v4_vec = v4set.iter().copied().collect::<Vec<_>>();
            let v6_vec = v6set.iter().copied().collect::<Vec<_>>();

            let (agg_v4, agg_v6) = join(|| IpNet::aggregate(&v4_vec), || IpNet::aggregate(&v6_vec));

            (cc, (agg_v4, agg_v6))
        })
        .collect();

    Ok(aggregated.into_iter().collect())
}

// 単一RIRテキストをパースし、国コード→(v4セット, v6セット)の部分結果を返す
fn parse_one_rir_text_to_sets(text: &str) -> Result<CountrySets, AppError> {
    text.lines().filter_map(RirAllocation::from_line).try_fold(
        CountrySets::new(),
        |mut countries, allocation| {
            let entry = countries
                .entry(allocation.country_code.to_uppercase())
                .or_default();
            let target = if allocation.ip_type == "ipv4" {
                &mut entry.0
            } else {
                &mut entry.1
            };
            target.extend(allocation.parse_nets()?);
            Ok(countries)
        },
    )
}
