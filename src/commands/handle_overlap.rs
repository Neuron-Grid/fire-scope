use crate::cli::Cli;
use crate::common::OutputFormat;
use crate::common_download::download_all_rir_files;
use crate::error::AppError;
use crate::output::write_overlap_to_file;
use crate::overlap::find_overlaps;
use crate::process::parse_and_collect_ips;
use ipnet::IpNet;
use reqwest::Client;
use std::collections::BTreeSet;

/// overlapモードのメイン処理
pub async fn run_overlap(
    args: &Cli,
    client: &Client,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let (country_codes, as_numbers) = validate_args(args)?;
    let (rir_texts_ok, failed_urls) = download_all_rir_files(client).await?;
    if !failed_urls.is_empty() {
        eprintln!("[Warning] The following RIR URLs failed to download:");
        for url in &failed_urls {
            eprintln!("  - {}", url);
        }
    }
    let (country_ips_v4, country_ips_v6) = collect_country_ips(&country_codes, &rir_texts_ok)?;
    let as_strings: Vec<String> = as_numbers.iter().map(|n| n.to_string()).collect();
    let (as_ips_v4, as_ips_v6) = collect_as_ips_no_rpki(client, &as_strings).await?;
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

/// 引数の検証
/// --overlapオプションが指定されている場合、--countryと--as-numberの両方が必要
fn validate_args(args: &Cli) -> Result<(Vec<String>, Vec<u32>), AppError> {
    let country_codes = args.country_codes.clone().ok_or_else(|| {
        AppError::InvalidInput("Error: --overlap requires --country <codes>".into())
    })?;
    let as_numbers = args.as_numbers.clone().ok_or_else(|| {
        AppError::InvalidInput("Error: --overlap requires --as-number <numbers>".into())
    })?;
    Ok((country_codes, as_numbers))
}

/// 国コードリストを1つずつparse_and_collect_ips()で取得
/// 国コードは大文字に変換してから渡す
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

/// AS番号リストを1つずつget_ips_for_as_once_no_rpki()で取得
async fn collect_as_ips_no_rpki(
    client: &Client,
    as_strings: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let mut a_v4 = BTreeSet::new();
    let mut a_v6 = BTreeSet::new();

    for asn in as_strings {
        let (v4set, v6set) = crate::asn::get_prefixes_via_rdap(client, asn).await?;
        a_v4.extend(v4set);
        a_v6.extend(v6set);
    }
    Ok((a_v4, a_v6))
}

/// 国コードとAS番号のIPリストを受け取り、重複部分を計算
/// IPv4とIPv6の重複部分をそれぞれ計算し、結果を結合して返す
fn calculate_overlaps(
    (country_v4, country_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
    (as_v4, as_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
) -> BTreeSet<IpNet> {
    let overlaps_v4 = find_overlaps(&country_v4, &as_v4);
    let overlaps_v6 = find_overlaps(&country_v6, &as_v6);
    overlaps_v4.into_iter().chain(overlaps_v6).collect()
}
