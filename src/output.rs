use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::ip::{IpFamily, IpSets};
use crate::output_common::{make_header, sanitize_identifier, write_list_nft, write_list_txt};
use chrono::Local;
use clap::ValueEnum;
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum OutputFormat {
    Txt,
    Nft,
}

impl OutputFormat {
    pub(crate) const fn extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Nft => "nft",
        }
    }
}

async fn write_list(
    path: &Path,
    ipnets: &BTreeSet<IpNet>,
    header: &str,
    format: OutputFormat,
) -> Result<(), AppError> {
    match format {
        OutputFormat::Txt => write_list_txt(path, ipnets, header).await,
        OutputFormat::Nft => write_list_nft(path, ipnets, header).await,
    }
}

pub(crate) async fn write_ip_lists_to_files(
    country_code: &str,
    ip_sets: &IpSets,
    format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let safe_code = sanitize_identifier(&country_code.to_ascii_uppercase());
    let header = make_header(&now, &safe_code, "N/A");
    let extension = format.extension();
    let ipv4_file = format!("IPv4_{safe_code}.{extension}");
    let ipv6_file = format!("IPv6_{safe_code}.{extension}");

    write_list(Path::new(&ipv4_file), ip_sets.ipv4(), &header, format).await?;
    write_list(Path::new(&ipv6_file), ip_sets.ipv6(), &header, format).await?;
    debug.log(format!("Wrote {ipv4_file} and {ipv6_file}"));
    Ok(())
}

pub(crate) async fn write_as_ip_list_to_file(
    as_number: u32,
    family: IpFamily,
    ipnets: &BTreeSet<IpNet>,
    format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let safe_as = sanitize_identifier(&as_number.to_string());
    let header = make_header(&now, "N/A", &safe_as);
    let extension = format.extension();
    let file_name = format!("AS_{safe_as}_{}.{extension}", family.as_str());

    write_list(Path::new(&file_name), ipnets, &header, format).await?;
    debug.log(format!("Wrote {file_name}"));
    Ok(())
}

pub(crate) async fn write_overlap_to_file(
    country_code: &str,
    as_numbers: &[u32],
    overlaps: &IpSets,
    format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    if overlaps.is_empty() {
        debug.log(format!(
            "No overlap found for country={country_code} and AS={}",
            format_as_numbers(as_numbers)
        ));
        return Ok(());
    }

    let safe_country = sanitize_identifier(country_code);
    let safe_as = sanitize_identifier(&format_as_numbers(as_numbers));
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let header = make_header(&now, &safe_country, &safe_as);
    let extension = format.extension();

    for (label, ipnets) in [
        (IpFamily::V4.as_str(), overlaps.ipv4()),
        (IpFamily::V6.as_str(), overlaps.ipv6()),
    ] {
        if !ipnets.is_empty() {
            let file_name = format!("overlap_{safe_country}_{safe_as}_{label}.{extension}");
            write_list(Path::new(&file_name), ipnets, &header, format).await?;
            debug.log(format!("Wrote {file_name}"));
        }
    }

    Ok(())
}

fn format_as_numbers(as_numbers: &[u32]) -> String {
    as_numbers
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::{OutputFormat, format_as_numbers};

    #[test]
    fn output_metadata_is_deterministic() {
        assert_eq!(OutputFormat::Txt.extension(), "txt");
        assert_eq!(OutputFormat::Nft.extension(), "nft");
        assert_eq!(format_as_numbers(&[1234, 5678]), "1234_5678");
    }
}
