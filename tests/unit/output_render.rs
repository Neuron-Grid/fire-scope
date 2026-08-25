use super::{make_header, nft_chunks, sanitize_identifier, txt_chunks};
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::error::Error;

fn sample_ipnets() -> Result<BTreeSet<IpNet>, Box<dyn Error>> {
    Ok([
        "192.0.2.0/24".parse::<IpNet>()?,
        "2001:db8::/32".parse::<IpNet>()?,
    ]
    .into_iter()
    .collect())
}

fn collect_chunks(chunks: impl Iterator<Item = String>) -> String {
    chunks.fold(String::new(), |mut output, chunk| {
        output.push_str(&chunk);
        output
    })
}

#[test]
fn renderers_preserve_txt_and_nft_bytes() -> Result<(), Box<dyn Error>> {
    let ipnets = sample_ipnets()?;
    let empty = BTreeSet::new();

    assert_eq!(
        collect_chunks(txt_chunks(&ipnets, "# header\n")),
        "# header\n192.0.2.0/24\n2001:db8::/32\n"
    );
    assert_eq!(
        collect_chunks(nft_chunks(&ipnets, "# header\n", "routes")),
        "# header\ndefine routes = {\n    192.0.2.0/24,\n    2001:db8::/32\n}\n"
    );
    assert_eq!(
        collect_chunks(txt_chunks(&empty, "# header\n")),
        "# header\n\n"
    );
    assert_eq!(
        collect_chunks(nft_chunks(&empty, "# header\n", "routes")),
        "# header\ndefine routes = {\n}\n"
    );
    Ok(())
}

#[test]
fn metadata_and_identifier_are_deterministic() {
    assert_eq!(
        make_header("2026-08-25 12:34:56", "JP", "N/A"),
        "# Generated at: 2026-08-25 12:34:56\n# Country Code: JP\n# AS Number: N/A\n\n"
    );
    assert_eq!(sanitize_identifier("---"), "UNKNOWN");
    assert_eq!(sanitize_identifier("--a/b--"), "a_b");
    assert_eq!(sanitize_identifier(&"a".repeat(64)), "a".repeat(64));
    assert_eq!(sanitize_identifier(&"a".repeat(65)), "a".repeat(64));
}
