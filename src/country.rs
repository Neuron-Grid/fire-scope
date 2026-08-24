use crate::error::AppError;
use crate::ip::IpSets;
use std::collections::HashMap;

pub(crate) type CountryMap = HashMap<String, IpSets>;

pub(crate) struct CountrySelection<'a> {
    pub(crate) found: Vec<(&'a str, &'a IpSets)>,
    pub(crate) missing_codes: Vec<&'a str>,
}

impl CountrySelection<'_> {
    pub(crate) fn merged_ips(&self) -> IpSets {
        self.found.iter().fold(IpSets::default(), |sets, (_, ips)| {
            sets.merge((*ips).clone())
        })
    }
}

pub(crate) async fn parse_country_map(
    rir_texts: &[String],
    country_codes: &[String],
) -> Result<CountryMap, AppError> {
    let texts = rir_texts.to_owned();
    let selected = country_codes.to_owned();
    tokio::task::spawn_blocking(move || crate::parse::parse_all_country_codes(&texts, &selected))
        .await?
}

pub(crate) fn select_country_ips<'a>(
    country_map: &'a CountryMap,
    country_codes: &'a [String],
) -> CountrySelection<'a> {
    country_codes.iter().fold(
        CountrySelection {
            found: Vec::new(),
            missing_codes: Vec::new(),
        },
        |mut selection, country_code| {
            match country_map.get(country_code) {
                Some(ips) => selection.found.push((country_code, ips)),
                None => selection.missing_codes.push(country_code),
            }
            selection
        },
    )
}

#[cfg(test)]
#[path = "../tests/unit/country.rs"]
mod tests;
