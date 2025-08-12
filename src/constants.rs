/// 定数の共通化

pub const RIR_URLS: &[&str] = &[
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest",
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
];

/// ダウンロード本文サイズ上限 (バイト)。防御的に 32 MiB
pub const MAX_RIR_DOWNLOAD_BYTES: u64 = 32 * 1024 * 1024;

/// JSON API 応答の最大サイズ上限 (バイト)。防御的に 8 MiB
pub const MAX_JSON_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024;
