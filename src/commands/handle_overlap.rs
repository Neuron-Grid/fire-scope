use crate::cli::Cli;
use crate::common::OutputFormat;
use crate::common_download::download_all_rir_files;
use crate::error::AppError;
use crate::output::write_overlap_to_file;
use crate::overlap::find_overlaps;
use crate::parse::parse_all_country_codes;
use ipnet::IpNet;
use reqwest::Client;
use std::collections::BTreeSet;
use crate::common::debug_log;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// overlapモードのメイン処理
pub async fn run_overlap(
    args: &Cli,
    client: &Client,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let (country_codes, as_numbers) = validate_args(args)?;
    let (rir_texts_ok, failed_urls) =
        download_all_rir_files(client, args.max_retries, args.max_backoff_sec).await?;
    if !failed_urls.is_empty() {
        debug_log(format!(
            "The following RIR URLs failed to download: {:?}",
            failed_urls
        ));
        if !args.continue_on_partial {
            return Err(AppError::Other(
                "Some RIR downloads failed (use --continue-on-partial to proceed)".into(),
            ));
        }
    }
    if rir_texts_ok.is_empty() {
        return Err(AppError::Other(
            "No RIR files available to process".into(),
        ));
    }
    let (country_ips_v4, country_ips_v6) = collect_country_ips(&country_codes, &rir_texts_ok)?;
    let as_strings: Vec<String> = as_numbers.iter().map(|n| n.to_string()).collect();
    let (as_ips_v4, as_ips_v6) =
        collect_as_ips_no_rpki(client, &as_strings, args.concurrency).await?;
    let overlap_nets = calculate_overlaps((country_ips_v4, country_ips_v6), (as_ips_v4, as_ips_v6));
    write_overlap_to_file(
        &country_codes.join("_").to_uppercase(),
        &as_strings.join("_"),
        &overlap_nets,
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
    // 一度だけ全RIRテキストをパースし、国コード→(IPv4, IPv6)のマップを作成
    let country_map = parse_all_country_codes(rir_texts)?;

    let mut c_v4 = BTreeSet::new();
    let mut c_v6 = BTreeSet::new();

    for code in country_codes {
        let upper = code.to_uppercase();
        if let Some((v4_vec, v6_vec)) = country_map.get(&upper) {
            c_v4.extend(v4_vec.iter().copied());
            c_v6.extend(v6_vec.iter().copied());
        } else {
            debug_log(format!("No IPs found for country code: {}", upper));
        }
    }

    Ok((c_v4, c_v6))
}

/// AS番号リストを1つずつget_ips_for_as_once_no_rpki()で取得
async fn collect_as_ips_no_rpki(
    client: &Client,
    as_strings: &[String],
    concurrency: usize,
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), AppError> {
    let max_concurrent = if concurrency == 0 { 1 } else { concurrency };
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    let mut handles = Vec::with_capacity(as_strings.len());
    for asn in as_strings.iter().cloned() {
        let client_c = client.clone();
        let sem_c = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem_c.acquire_owned().await?;
            crate::asn::get_prefixes_via_rdap(&client_c, &asn).await
        }));
    }

    let mut a_v4 = BTreeSet::new();
    let mut a_v6 = BTreeSet::new();

    for h in handles {
        // JoinError は AppError に伝播
        let res = h.await??;
        let (v4set, v6set) = res;
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
