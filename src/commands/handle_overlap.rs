use crate::asn::get_ips_for_as;
use crate::cli::Cli;
use crate::common::IpFamily;
use crate::fetch::fetch_with_retry;
use crate::output::write_overlap_to_file;
use crate::overlap::find_overlaps;
use crate::process::parse_and_collect_ips;
use ipnet::IpNet;
use reqwest::Client;
use std::collections::BTreeSet;
use std::error::Error;

/// RIRファイルURL
const RIR_URLS: &[&str] = &[
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest",
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
];

/// --overlap が指定された場合に呼ばれる処理
pub async fn run_overlap(
    args: &Cli, // main.rs から受け取った Cli 構造体
    client: &Client,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (country_codes, as_numbers) = validate_args(args)?;
    let rir_texts = download_all_rir_files(client).await?;

    // 1. 国コードからIP集合を収集
    let (country_ips_v4, country_ips_v6) = collect_country_ips(&country_codes, &rir_texts)?;

    // 2. AS番号からIP集合を収集
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();
    let (as_ips_v4, as_ips_v6) = collect_as_ips(&as_strings).await?;

    // 3. オーバーラップを計算
    let overlap_nets = calculate_overlaps((country_ips_v4, country_ips_v6), (as_ips_v4, as_ips_v6));

    // 4. 結果をファイルに書き出し
    write_overlap_result(&country_codes, &as_strings, &overlap_nets, &args.mode)?;
    Ok(())
}

/// 引数バリデーション
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

/// RIRファイルの一括ダウンロード
async fn download_all_rir_files(
    client: &Client,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    use futures::future::join_all;

    let mut handles = Vec::new();
    for url in RIR_URLS {
        let url_owned = url.to_string();
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            fetch_with_retry(&c, &url_owned).await
        }));
    }

    let results = join_all(handles).await;
    let mut rir_texts = Vec::new();
    for r in results {
        match r {
            Ok(Ok(text)) => rir_texts.push(text),
            Ok(Err(e)) => eprintln!("HTTPエラー: {}", e),
            Err(e) => eprintln!("タスク失敗: {}", e),
        }
    }
    Ok(rir_texts)
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

/// AS番号からIPを収集
async fn collect_as_ips(
    as_strings: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn Error + Send + Sync>> {
    let mut a_v4 = BTreeSet::new();
    let mut a_v6 = BTreeSet::new();

    for asn in as_strings {
        let v4set = get_ips_for_as(asn, IpFamily::V4).await?;
        let v6set = get_ips_for_as(asn, IpFamily::V6).await?;
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
    write_overlap_to_file(&combined_country, &combined_asn, overlap_nets, mode)?;
    Ok(())
}
