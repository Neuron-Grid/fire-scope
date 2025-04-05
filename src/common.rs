use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum IpFamily {
    V4,
    V6,
}

impl FromStr for IpFamily {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IPv4" => Ok(IpFamily::V4),
            "IPv6" => Ok(IpFamily::V6),
            _ => Err("Invalid IP version. Must be 'IPv4' or 'IPv6'"),
        }
    }
}

impl IpFamily {
    /// whoisのrouteキーを返す ("route:" / "route6:")
    pub fn route_key(self) -> &'static str {
        match self {
            IpFamily::V4 => "route:",
            IpFamily::V6 => "route6:",
        }
    }

    /// ログやファイル名で使うラベル用
    pub fn as_str(self) -> &'static str {
        match self {
            IpFamily::V4 => "IPv4",
            IpFamily::V6 => "IPv6",
        }
    }
}

/// 出力形式を管理するためのenum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Txt,
    Nft,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "nft" => OutputFormat::Nft,
            _ => OutputFormat::Txt, // デフォルトは Txt
        }
    }
}
