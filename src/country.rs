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
mod tests {
    use super::{CountryMap, select_country_ips};
    use crate::ip::IpSets;
    use ipnet::IpNet;
    use std::str::FromStr;

    fn ipnet(cidr: &str) -> Option<IpNet> {
        IpNet::from_str(cidr).ok()
    }

    #[test]
    fn selects_and_merges_countries_without_changing_the_map() {
        let jp = [ipnet("192.0.2.0/24")]
            .into_iter()
            .flatten()
            .collect::<IpSets>();
        let us = [ipnet("2001:db8::/32")]
            .into_iter()
            .flatten()
            .collect::<IpSets>();
        let country_map = CountryMap::from([("JP".to_string(), jp), ("US".to_string(), us)]);
        let original = country_map.clone();
        let country_codes = ["JP".into(), "US".into(), "ZZ".into()];

        let selection = select_country_ips(&country_map, &country_codes);
        let merged = selection.merged_ips();

        assert_eq!(country_map, original);
        assert_eq!(merged.ipv4().len(), 1);
        assert_eq!(merged.ipv6().len(), 1);
        assert_eq!(selection.missing_codes, ["ZZ"]);
    }
}
