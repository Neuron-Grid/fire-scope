use ipnet::IpNet;
use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};

/// IPv4/IPv6 の Vec ペア（parse 結果など）
pub type IpVecPair = (Vec<IpNet>, Vec<IpNet>);

/// IPv4/IPv6 の BTreeSet ペア（重複排除済み）
pub type IpSetPair = (BTreeSet<IpNet>, BTreeSet<IpNet>);

pub(crate) fn partition_ipnets(nets: impl IntoIterator<Item = IpNet>) -> IpSetPair {
    nets.into_iter()
        .partition(|net| matches!(net, IpNet::V4(_)))
}

pub(crate) fn aggregate_ipnets(nets: impl IntoIterator<Item = IpNet>) -> IpSetPair {
    partition_ipnets(IpNet::aggregate(&nets.into_iter().collect::<Vec<_>>()))
}

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

/// デバッグの有効/無効を設定
pub fn set_debug(enabled: bool) {
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
}

/// デバッグが有効かどうか
pub fn debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

/// デバッグ出力（stderr）
pub fn debug_log(msg: impl AsRef<str>) {
    if debug_enabled() {
        eprintln!("[debug] {}", msg.as_ref());
    }
}

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
    /// ファイル拡張子を返す
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Txt => "txt",
            OutputFormat::Nft => "nft",
        }
    }
}

// ここで標準トレイト `FromStr` を実装し、文字列 => `OutputFormat` 変換を行う
impl FromStr for OutputFormat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nft" => Ok(OutputFormat::Nft),
            "txt" | "" => Ok(OutputFormat::Txt),
            _ => Err("Invalid output format. Valid options: 'txt' or 'nft'"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{aggregate_ipnets, partition_ipnets};
    use ipnet::IpNet;
    use std::str::FromStr;

    fn net(cidr: &str) -> Option<IpNet> {
        IpNet::from_str(cidr).ok()
    }

    #[test]
    fn ipnet_helpers_partition_and_aggregate_without_changing_input() {
        let input = [
            net("192.0.2.0/25"),
            net("192.0.2.128/25"),
            net("2001:db8::/32"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let original = input.clone();

        let (partitioned_v4, partitioned_v6) = partition_ipnets(input.iter().copied());
        let (aggregated_v4, aggregated_v6) = aggregate_ipnets(input.iter().copied());

        assert_eq!(input, original);
        assert_eq!(partitioned_v4.len(), 2);
        assert_eq!(partitioned_v6.len(), 1);
        assert_eq!(
            aggregated_v4
                .iter()
                .next()
                .map(ToString::to_string)
                .as_deref(),
            Some("192.0.2.0/24")
        );
        assert_eq!(aggregated_v6, partitioned_v6);
    }
}
