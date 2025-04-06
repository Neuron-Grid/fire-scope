use crate::asn::get_ips_for_as_once;
use crate::cli::Cli;
use crate::common::OutputFormat;
use crate::output::write_overlap_to_file;
use crate::overlap::find_overlaps;
use crate::process::parse_and_collect_ips;
use crate::rir_download::download_all_rir_files;
use ipnet::IpNet;
use reqwest::Client;
use std::collections::BTreeSet;
use std::error::Error;

/// --overlap が指定された場合に呼ばれる処理
pub async fn run_overlap(
    args: &Cli,
    client: &Client,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (country_codes, as_numbers) = validate_args(args)?;
    let rir_texts = download_all_rir_files(client).await?;

    // 国コードIP収集
    let (country_ips_v4, country_ips_v6) = collect_country_ips(&country_codes, &rir_texts)?;

    // AS番号からIP集合を収集
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();
    let (as_ips_v4, as_ips_v6) = collect_as_ips(&as_strings).await?;

    // 重複を探す
    let overlap_nets = calculate_overlaps((country_ips_v4, country_ips_v6), (as_ips_v4, as_ips_v6));

    // 結果をファイル出力
    write_overlap_result(
        &country_codes,
        &as_strings,
        &overlap_nets,
        &args.mode,
        output_format,
    )?;
    Ok(())
}

fn validate_args(args: &Cli) -> Result<(Vec<String>, Vec<u32>), Box<dyn Error + Send + Sync>> {
    let country_codes = args
        .country_codes
        .clone()
        .ok_or("Error: --overlap requires --country <codes>")?;
    let as_numbers = args
        .as_numbers
        .clone()
        .ok_or("Error: --overlap requires --as-number <numbers>")?;
    Ok((country_codes, as_numbers))
}

/// 国コードからIPを収集
fn collect_country_ips(
    country_codes: &[String],
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn Error + Send + Sync>> {
    let mut c_v4 = BTreeSet::new();
    let mut c_v6 = BTreeSet::new();

    for code in country_codes {
        let (v4, v6) = parse_and_collect_ips(&code.to_uppercase(), rir_texts)?;
        c_v4.extend(v4);
        c_v6.extend(v6);
    }
    Ok((c_v4, c_v6))
}

/// AS番号からIPを収集 (1回のwhois呼び出しでv4/v6同時取得)
async fn collect_as_ips(
    as_strings: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn Error + Send + Sync>> {
    let mut a_v4 = BTreeSet::new();
    let mut a_v6 = BTreeSet::new();

    for asn in as_strings {
        // get_ips_for_as_onceは自作の関数
        let (v4set, v6set) = get_ips_for_as_once(asn).await?;
        a_v4.extend(v4set);
        a_v6.extend(v6set);
    }
    Ok((a_v4, a_v6))
}

/// オーバーラップ計算
fn calculate_overlaps(
    (country_v4, country_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
    (as_v4, as_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
) -> BTreeSet<IpNet> {
    let overlaps_v4 = find_overlaps(&country_v4, &as_v4);
    let overlaps_v6 = find_overlaps(&country_v6, &as_v6);
    overlaps_v4.into_iter().chain(overlaps_v6).collect()
}

/// ファイル出力
fn write_overlap_result(
    countries: &[String],
    as_strings: &[String],
    overlap_nets: &BTreeSet<IpNet>,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if overlap_nets.is_empty() {
        println!(
            "[overlap] No overlap found for countries={:?} and AS numbers={:?}",
            countries, as_strings
        );
        return Ok(());
    }

    let combined_country = countries.join("_").to_uppercase();
    let combined_asn = as_strings.join("_");
    write_overlap_to_file(
        &combined_country,
        &combined_asn,
        overlap_nets,
        mode,
        output_format,
    )?;
    Ok(())
}
