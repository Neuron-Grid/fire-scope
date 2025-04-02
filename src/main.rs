use clap::Parser;
use fire_scope::common::IpFamily;
use fire_scope::{
    asn::{get_ips_for_as, process_as_numbers},
    fetch::fetch_with_retry,
    output::write_overlap_to_file,
    overlap::find_overlaps,
    process::{parse_and_collect_ips, process_country_code},
};
use ipnet::IpNet;
use reqwest::Client;
use std::collections::BTreeSet;
use tokio::task::JoinHandle;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "This tool can be used to obtain IP addresses by country or by AS number."
)]
struct Cli {
    #[arg(
        short = 'c',
        long = "country",
        required_unless_present_any = ["as_numbers", "overlap"],
        required = false,
        num_args = 1..,
        help = "Specify the country codes.\nExample: jp br us"
    )]
    country_codes: Option<Vec<String>>,

    #[arg(
        short = 'a',
        long = "as-number",
        required_unless_present_any = ["country_codes", "overlap"],
        required = false,
        num_args = 1..,
        help = "Specify AS numbers.\nExample: AS0000 AS1234"
    )]
    as_numbers: Option<Vec<String>>,

    #[arg(
        short = 'm',
        long = "mode",
        default_value = "overwrite",
        required = false,
        hide_default_value = true,
        help = "Select file output mode: 'append' or 'overwrite'\ndefault: overwrite"
    )]
    mode: String,

    #[arg(
        long = "overlap",
        help = "If both country code(s) and AS number(s) are specified, computes overlap only.",
        required = false,
        default_value = "false"
    )]
    overlap: bool,
}

/// エントリポイント
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Cli::parse();
    run(args).await
}

/// メインの処理を振り分ける関数
async fn run(args: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    // overlap指定時の処理
    if args.overlap {
        handle_overlap(&args, &client).await?;
        return Ok(());
    }
    // AS番号の処理
    if let Some(as_list) = &args.as_numbers {
        process_as_numbers(as_list, &args.mode).await?;
        return Ok(());
    }
    // 国コードの処理
    if let Some(country_codes) = &args.country_codes {
        handle_country_codes(country_codes, &client, &args.mode).await?;
        return Ok(());
    }
    // どの引数も指定されなかった場合
    eprintln!("Error: Please specify --country or --as-number.\nUse --help for usage.");
    Ok(())
}

/// --overlapが指定された場合の処理
async fn handle_overlap(
    args: &Cli,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 引数チェック
    // 国コードとAS番号が無い場合はエラー
    let (country_codes, as_numbers) = validate_overlap_args(args)?;
    // RIRファイルをまとめてダウンロード
    let rir_texts = download_all_rir_files(client, RIR_URLS).await?;
    //  国コードからIP集合を作成
    let (country_ips_v4, country_ips_v6) = collect_country_ips(&country_codes, &rir_texts)?;
    // AS番号からIP集合を作成
    let (as_ips_v4, as_ips_v6) = collect_as_ips(&as_numbers).await?;
    // 重複（オーバーラップ）を計算
    let overlaps = calculate_overlaps((country_ips_v4, country_ips_v6), (as_ips_v4, as_ips_v6));
    // 結果をファイルへ書き出し
    write_overlap_result(&country_codes, &as_numbers, &overlaps, &args.mode)?;

    Ok(())
}

/// 重複（オーバーラップ）に必要な引数（国コードとAS番号）をチェックする関数
fn validate_overlap_args(
    args: &Cli,
) -> Result<(Vec<String>, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    let country_codes = match &args.country_codes {
        Some(c) => c.clone(),
        None => {
            eprintln!("Error: --overlap requires --country <codes> and --as-number <numbers>");
            return Err("Missing country codes".into());
        }
    };
    let as_numbers = match &args.as_numbers {
        Some(a) => a.clone(),
        None => {
            eprintln!("Error: --overlap requires --country <codes> and --as-number <numbers>");
            return Err("Missing as numbers".into());
        }
    };
    Ok((country_codes, as_numbers))
}

/// 国コードからIPを収集する
fn collect_country_ips(
    country_codes: &[String],
    rir_texts: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn std::error::Error + Send + Sync>> {
    let mut country_ips_v4 = BTreeSet::new();
    let mut country_ips_v6 = BTreeSet::new();

    for code in country_codes {
        let upper_code = code.to_uppercase();
        let (v4set, v6set) = parse_and_collect_ips(&upper_code, rir_texts)?;
        country_ips_v4.extend(v4set);
        country_ips_v6.extend(v6set);
    }
    Ok((country_ips_v4, country_ips_v6))
}

/// AS番号からIPを収集する
async fn collect_as_ips(
    as_numbers: &[String],
) -> Result<(BTreeSet<IpNet>, BTreeSet<IpNet>), Box<dyn std::error::Error + Send + Sync>> {
    let mut as_ips_v4 = BTreeSet::new();
    let mut as_ips_v6 = BTreeSet::new();

    for asn in as_numbers {
        let set_v4 = get_ips_for_as(asn, IpFamily::V4).await?;
        let set_v6 = get_ips_for_as(asn, IpFamily::V6).await?;
        as_ips_v4.extend(set_v4);
        as_ips_v6.extend(set_v6);
    }
    Ok((as_ips_v4, as_ips_v6))
}

/// 国コード由来とAS番号由来のIPセットからオーバーラップを計算する
fn calculate_overlaps(
    (country_ips_v4, country_ips_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
    (as_ips_v4, as_ips_v6): (BTreeSet<IpNet>, BTreeSet<IpNet>),
) -> BTreeSet<IpNet> {
    // v4同士の重複とv6同士の重複を取得
    let overlaps_v4 = find_overlaps(&country_ips_v4, &as_ips_v4);
    let overlaps_v6 = find_overlaps(&country_ips_v6, &as_ips_v6);

    // 合算して返す
    overlaps_v4
        .into_iter()
        .chain(overlaps_v6)
        .collect::<BTreeSet<IpNet>>()
}

/// オーバーラップ結果をファイルに書き出す
fn write_overlap_result(
    country_codes: &[String],
    as_numbers: &[String],
    overlaps: &BTreeSet<IpNet>,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let combined_countries = country_codes.join("_").to_uppercase();
    let combined_asn = as_numbers.join("_");
    write_overlap_to_file(&combined_countries, &combined_asn, overlaps, mode)?;
    Ok(())
}

/// RIRファイルをすべてダウンロードしてメモリ上に文字列ベクタとして返す
async fn download_all_rir_files(
    client: &Client,
    urls: &[&str],
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    use futures::future::join_all;
    let mut handles = Vec::new();

    for url in urls {
        let url_owned = url.to_string();
        let client_ref = client.clone();
        handles.push(tokio::spawn(async move {
            fetch_with_retry(&client_ref, &url_owned).await
        }));
    }

    let results = join_all(handles).await;
    let mut rir_texts = Vec::new();

    for res in results {
        match res {
            Ok(Ok(text)) => {
                rir_texts.push(text);
            }
            Ok(Err(e)) => {
                eprintln!("HTTP取得エラー: {}", e);
            }
            Err(e) => {
                eprintln!("タスク失敗: {}", e);
            }
        }
    }

    Ok(rir_texts)
}

/// --countryが指定された場合の処理
async fn handle_country_codes(
    country_codes: &[String],
    client: &Client,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rir_texts = download_all_rir_files(client, RIR_URLS).await?;

    let mut tasks: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> =
        Vec::new();
    for code in country_codes {
        let rir_texts_clone = rir_texts.clone();
        let mode_clone = mode.to_string();
        let code_clone = code.to_uppercase();

        let handle = tokio::spawn(async move {
            if let Err(e) = process_country_code(&code_clone, &rir_texts_clone, &mode_clone).await {
                eprintln!("エラー (国コード: {}): {}", code_clone, e);
            }
            Ok(())
        });
        tasks.push(handle);
    }

    for t in tasks {
        let _ = t.await?;
    }
    Ok(())
}

/// RIRファイルをダウンロードする際に使用するURLのリスト
const RIR_URLS: &[&str] = &[
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest",
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
];
