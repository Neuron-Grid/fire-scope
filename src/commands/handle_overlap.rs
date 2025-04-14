use crate::asn::get_ips_for_as_once;
use crate::cli::Cli;
use crate::common::OutputFormat;
use crate::error::AppError;
use crate::output::write_overlap_to_file;
use crate::overlap::find_overlaps;
use crate::process::parse_and_collect_ips;
use crate::rir_download::download_all_rir_files;
use ipnet::IpNet;
use reqwest::Client;
use std::collections::BTreeSet;

pub async fn run_overlap(
    args: &Cli,
    client: &Client,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let (country_codes, as_numbers) = validate_args(args)?;
    let rir_texts = download_all_rir_files(client).await?;

    let (country_ips_v4, country_ips_v6) = collect_country_ips(&country_codes, &rir_texts)?;
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();
    let (as_ips_v4, as_ips_v6) = collect_as_ips(&as_strings).await?;

    let overlap_nets = calculate_overlaps((country_ips_v4, country_ips_v6), (as_ips_v4, as_ips_v6));

    write_overlap_to_file(
        &country_codes.join("_").to_uppercase(),
        &as_strings.join("_"),
        &overlap_nets,
        &args.mode,
        output_format,
    )
    .await?;
    Ok(())
}

fn validate_args(args: &Cli) -> Result<(Vec<String>, Vec<u32>), AppError> {
    let country_codes = args.country_codes.clone().ok_or_else(|| {
        AppError::InvalidInput("Error: --overlap requires --country <codes>".into())
    })?;
    let as_numbers = args.as_numbers.clone().ok_or_else(|| {
        AppError::InvalidInput("Error: --overlap requires --as-number <numbers>".into())
    })?;
    Ok((country_codes, as_numbers))
}

fn collect_country_ips(
    country_codes: &[String],
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut c_v4 = BTreeSet::new();
    let mut c_v6 = BTreeSet::new();

    for code in country_codes {
        let (v4, v6) = parse_and_collect_ips(&code.to_uppercase(), rir_texts)?;
        c_v4.extend(v4);
        c_v6.extend(v6);
    }
    Ok((c_v4, c_v6))
}

async fn collect_as_ips(
    as_strings: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut a_v4 = BTreeSet::new();
    let mut a_v6 = BTreeSet::new();

    for asn in as_strings {
        let (v4set, v6set) = get_ips_for_as_once(asn).await?;
        a_v4.extend(v4set);
        a_v6.extend(v6set);
    }
    Ok((a_v4, a_v6))
}

fn calculate_overlaps(
    (country_v4, country_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
    (as_v4, as_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
) -> BTreeSet<IpNet> {
    let overlaps_v4 = find_overlaps(&country_v4, &as_v4);
    let overlaps_v6 = find_overlaps(&country_v6, &as_v6);
    overlaps_v4.into_iter().chain(overlaps_v6).collect()
}
