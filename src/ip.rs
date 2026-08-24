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
#[path = "../tests/unit/ip.rs"]
mod tests;
