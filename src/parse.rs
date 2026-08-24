use crate::error::AppError;
use crate::ip::{IpFamily, IpSets};
use ipnet::{IpNet, Ipv6Net};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

type CountrySets = HashMap<String, IpSets>;

#[derive(Debug, Clone, Copy)]
struct RirAllocation<'a> {
    country_code: &'a str,
    family: IpFamily,
    start: &'a str,
    value: &'a str,
}

impl<'a> RirAllocation<'a> {
    fn from_line(line: &'a str) -> Option<Self> {
        if line.starts_with('#') {
            return None;
        }

        let mut fields = line.split('|');
        let _registry = fields.next()?;
        let country_code = fields.next()?;
        let family = IpFamily::from_str(fields.next()?).ok()?;
        let start = fields.next()?;
        let value = fields.next()?;
        let _date = fields.next()?;
        let status = fields.next()?;

        (status.eq_ignore_ascii_case("allocated") || status.eq_ignore_ascii_case("assigned"))
            .then_some(Self {
                country_code,
                family,
                start,
                value,
            })
    }

    fn parse_nets(self) -> Result<Vec<IpNet>, AppError> {
        match self.family {
            IpFamily::V4 => crate::ipv4_utils::parse_ipv4_range_to_cidrs(self.start, self.value),
            IpFamily::V6 => parse_ipv6_range(self.start, self.value),
        }
    }
}

fn parse_ipv6_range(start: &str, prefix: &str) -> Result<Vec<IpNet>, AppError> {
    let cidr = format!("{start}/{prefix}");
    cidr.parse::<Ipv6Net>()
        .map(|net| vec![IpNet::V6(net)])
        .map_err(|error| AppError::ParseError(format!("IPv6 network parse error: {error}")))
}

pub(crate) fn parse_all_country_codes(
    rir_texts: &[String],
    country_codes: &[String],
) -> Result<CountrySets, AppError> {
    let selected = country_codes
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let partials = rir_texts
        .par_iter()
        .map(|text| parse_one_rir_text(text, &selected))
        .collect::<Result<Vec<_>, AppError>>()?;

    let merged = partials.into_iter().flatten().fold(
        CountrySets::new(),
        |mut countries, (country_code, sets)| {
            let entry = countries.entry(country_code).or_default();
            *entry = std::mem::take(entry).merge(sets);
            countries
        },
    );

    Ok(merged
        .into_par_iter()
        .map(|(country_code, sets)| (country_code, sets.aggregated()))
        .collect())
}

fn parse_one_rir_text(
    text: &str,
    selected_country_codes: &HashSet<&str>,
) -> Result<CountrySets, AppError> {
    text.lines().filter_map(RirAllocation::from_line).try_fold(
        CountrySets::new(),
        |mut countries, allocation| {
            let country_code = allocation.country_code.to_ascii_uppercase();
            if !selected_country_codes.contains(country_code.as_str()) {
                return Ok(countries);
            }
            let parsed = allocation.parse_nets()?.into_iter().collect();
            let entry = countries.entry(country_code).or_default();
            *entry = std::mem::take(entry).merge(parsed);
            Ok(countries)
        },
    )
}

#[cfg(test)]
#[path = "../tests/unit/parse.rs"]
mod tests;
