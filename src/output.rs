use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::ip::{IpFamily, IpSets};
use crate::output_file::atomic_write;
use crate::output_render::{make_header, nft_chunks, sanitize_identifier, txt_chunks};
use chrono::Local;
use clap::ValueEnum;
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::num::NonZeroU32;
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
        OutputFormat::Txt => atomic_write(path, txt_chunks(ipnets, header)).await,
        OutputFormat::Nft => {
            let define_name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .map_or_else(|| "unknown_define".to_owned(), sanitize_identifier);
            atomic_write(path, nft_chunks(ipnets, header, &define_name)).await
        }
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

pub(crate) async fn write_as_ip_lists_to_files(
    as_number: NonZeroU32,
    ip_sets: &IpSets,
    format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    write_as_ip_list_to_file(as_number, IpFamily::V4, ip_sets.ipv4(), &now, format, debug).await?;
    write_as_ip_list_to_file(as_number, IpFamily::V6, ip_sets.ipv6(), &now, format, debug).await
}

async fn write_as_ip_list_to_file(
    as_number: NonZeroU32,
    family: IpFamily,
    ipnets: &BTreeSet<IpNet>,
    now: &str,
    format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    if ipnets.is_empty() {
        debug.log(format!("No {} routes for AS{as_number}", family.as_str()));
        return Ok(());
    }

    let safe_as = sanitize_identifier(&as_number.to_string());
    let header = make_header(now, "N/A", &safe_as);
    let extension = format.extension();
    let file_name = format!("AS_{safe_as}_{}.{extension}", family.as_str());

    write_list(Path::new(&file_name), ipnets, &header, format).await?;
    debug.log(format!("Wrote {file_name}"));
    Ok(())
}

pub(crate) async fn write_overlap_to_file(
    country_code: &str,
    as_numbers: &[NonZeroU32],
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

fn format_as_numbers(as_numbers: &[NonZeroU32]) -> String {
    as_numbers.iter().fold(String::new(), |mut output, number| {
        if !output.is_empty() {
            output.push('_');
        }
        output.push_str(&number.to_string());
        output
    })
}

#[cfg(test)]
#[path = "../tests/unit/output.rs"]
mod tests;
