use ipnet::IpNet;
use std::collections::{BTreeSet, btree_set};
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct IpSets {
    ipv4: BTreeSet<IpNet>,
    ipv6: BTreeSet<IpNet>,
}

impl IpSets {
    pub(crate) fn ipv4(&self) -> &BTreeSet<IpNet> {
        &self.ipv4
    }

    pub(crate) fn ipv6(&self) -> &BTreeSet<IpNet> {
        &self.ipv6
    }

    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.ipv4.extend(other.ipv4);
        self.ipv6.extend(other.ipv6);
        self
    }

    pub(crate) fn aggregated(self) -> Self {
        let nets = self.into_iter().collect::<Vec<_>>();
        IpNet::aggregate(&nets).into_iter().collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.ipv4.is_empty() && self.ipv6.is_empty()
    }
}

impl FromIterator<IpNet> for IpSets {
    fn from_iter<T: IntoIterator<Item = IpNet>>(iter: T) -> Self {
        let (ipv4, ipv6) = iter
            .into_iter()
            .partition(|net| matches!(net, IpNet::V4(_)));
        Self { ipv4, ipv6 }
    }
}

impl IntoIterator for IpSets {
    type Item = IpNet;
    type IntoIter = std::iter::Chain<btree_set::IntoIter<IpNet>, btree_set::IntoIter<IpNet>>;

    fn into_iter(self) -> Self::IntoIter {
        self.ipv4.into_iter().chain(self.ipv6)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IpFamily {
    V4,
    V6,
}

impl FromStr for IpFamily {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("ipv4") {
            Ok(Self::V4)
        } else if value.eq_ignore_ascii_case("ipv6") {
            Ok(Self::V6)
        } else {
            Err("IP family must be 'IPv4' or 'IPv6'")
        }
    }
}

impl IpFamily {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::V4 => "IPv4",
            Self::V6 => "IPv6",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IpFamily, IpSets};
    use ipnet::IpNet;
    use std::error::Error;
    use std::str::FromStr;

    #[test]
    fn sets_partition_merge_and_aggregate_without_changing_inputs() -> Result<(), Box<dyn Error>> {
        let first = [
            IpNet::from_str("192.0.2.0/25")?,
            IpNet::from_str("2001:db8::/32")?,
        ]
        .into_iter()
        .collect::<IpSets>();
        let second = [IpNet::from_str("192.0.2.128/25")?]
            .into_iter()
            .collect::<IpSets>();
        let first_before = first.clone();
        let second_before = second.clone();

        let aggregated = first.clone().merge(second.clone()).aggregated();

        assert_eq!(first, first_before);
        assert_eq!(second, second_before);
        assert_eq!(aggregated.ipv4().len(), 1);
        assert_eq!(
            aggregated.ipv4().iter().next().map(ToString::to_string),
            Some("192.0.2.0/24".to_owned())
        );
        assert_eq!(aggregated.ipv6(), first.ipv6());
        assert!(!aggregated.is_empty());
        assert!(IpSets::default().is_empty());
        Ok(())
    }

    #[test]
    fn family_parsing_ignores_ascii_case() {
        assert_eq!(IpFamily::from_str("ipv4"), Ok(IpFamily::V4));
        assert_eq!(IpFamily::from_str("IPV6"), Ok(IpFamily::V6));
        assert!(IpFamily::from_str("asn").is_err());
        assert_eq!(IpFamily::V4.as_str(), "IPv4");
        assert_eq!(IpFamily::V6.as_str(), "IPv6");
    }
}
