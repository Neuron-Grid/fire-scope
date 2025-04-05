use crate::common::OutputFormat;
use crate::constants::RIR_URLS;
use crate::fetch::fetch_with_retry;
use crate::process::process_all_country_codes;
use reqwest::Client;
use std::error::Error;

/// 国コード指定時の処理
pub async fn run_country_codes(
    country_codes: &[String],
    client: &Client,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // RIRファイルをすべてダウンロードしてメモリ上に保持
    let rir_texts = download_all_rir_files(client, RIR_URLS).await?;

    // 全ての国コードに対するIPアドレスを一度にパースして処理
    if let Err(e) = process_all_country_codes(country_codes, &rir_texts, mode, output_format).await
    {
        eprintln!("国コード処理中にエラーが発生しました: {}", e);
        return Err(e);
    }

    Ok(())
}

/// RIRファイルをすべてダウンロードして文字列ベクタとして返す
async fn download_all_rir_files(
    client: &Client,
    urls: &[&str],
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    use futures::future::join_all;

    let mut handles = Vec::new();
    for url in urls {
        let url_owned = url.to_string();
        let client_clone = client.clone();
        // 非同期で複数URLを並列取得
        handles.push(tokio::spawn(async move {
            fetch_with_retry(&client_clone, &url_owned).await
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
