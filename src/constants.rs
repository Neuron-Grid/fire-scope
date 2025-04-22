/// 定数の共通化

pub const RIR_URLS: &[&str] = &[
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest",
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
];

pub const RDAP_BASE_URLS: &[&str] = &[
    "https://rdap.arin.net/registry",
    "https://rdap.db.ripe.net",
    "https://rdap.apnic.net",
    "https://rdap.lacnic.net/rdap",
    "https://rdap.afrinic.net/rdap",
];

pub const TAL_URLS: &[&str] = &[
    "http://rpki.afrinic.net/tal/afrinic.tal",
    "https://www.lacnic.net/innovaportal/file/4983/1/lacnic.tal",
    "https://tal.rpki.ripe.net/ripe-ncc.tal",
    "https://tal.apnic.net/tal-archive/apnic-rfc6490-https.tal",
    "https://www.arin.net/resources/manage/rpki/arin.tal",
];
