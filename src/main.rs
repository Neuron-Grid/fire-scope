use clap::Parser;
use fetch_ccip::asn::process_as_numbers;
use fetch_ccip::fetch::fetch_with_retry;
use fetch_ccip::process::process_country_code;
use reqwest::Client;
use tokio::task::JoinHandle;

/// コマンドライン引数を処理するための構造体
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
        required_unless_present = "as_numbers",
        required = false,
        num_args = 1..,
        help = "Specify the country codes.\nExample: jp br us"
    )]
    country_codes: Option<Vec<String>>,

    #[arg(
        short = 'a',
        long = "asnumber",
        required_unless_present = "country_codes",
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
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // コマンドライン引数を取得
    let args = Cli::parse();
    // 実行処理を委譲
    run(args).await
}

/// アプリケーションのメインロジック
async fn run(args: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // HTTPクライアント生成
    let client = Client::new();

    // --asn オプションが指定された場合: AS番号ベースでルート取得
    if let Some(as_list) = &args.as_numbers {
        // AS番号を処理
        process_as_numbers(as_list, &args.mode).await?;
        return Ok(());
    }

    // --country オプションが指定された場合: 既存の国コード処理
    if let Some(country_codes) = &args.country_codes {
        // RIRファイルをすべてダウンロード
        let rir_texts = download_all_rir_files(&client, &RIR_URLS).await?;

        // 各国コードごとに処理を並行実行
        let mut tasks: Vec<JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>> =
            vec![];
        for code in country_codes {
            let rir_texts_clone = rir_texts.clone();
            let mode_clone = args.mode.clone();
            let code_clone = code.to_uppercase();

            let handle = tokio::spawn(async move {
                if let Err(e) =
                    process_country_code(&code_clone, &rir_texts_clone, &mode_clone).await
                {
                    eprintln!("エラー (国コード: {}): {}", code_clone, e);
                }
                Ok(())
            });
            tasks.push(handle);
        }

        // 全タスク終了を待機
        for t in tasks {
            let _ = t.await?;
        }
        return Ok(());
    }

    // 両方指定されなかった場合はエラーか、あるいはヘルプを表示
    eprintln!("Error: Please specify --country or --asn.\nUse --help for usage.");
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

const RIR_URLS: &[&str] = &[
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest",
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
];
