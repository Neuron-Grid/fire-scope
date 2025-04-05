use crate::process::process_country_code;
use crate::{constants::RIR_URLS, fetch::fetch_with_retry};
use reqwest::Client;
use std::error::Error;
use tokio::task::JoinHandle;

/// 国コード指定時の処理
pub async fn run_country_codes(
    country_codes: &[String],
    client: &Client,
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // RIRファイルをすべてダウンロードしてメモリ上に保持
    let rir_texts = download_all_rir_files(client, RIR_URLS).await?;

    // 国コードごとに非同期タスクを起動
    let mut tasks: Vec<JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>> = Vec::new();
    for code in country_codes {
        let rir_clone = rir_texts.clone();
        let mode_clone = mode.to_string();
        let upper_code = code.to_uppercase();
        let handle = tokio::spawn(async move {
            // 1国コード分の処理
            if let Err(e) = process_country_code(&upper_code, &rir_clone, &mode_clone).await {
                eprintln!("Error (country={}): {}", upper_code, e);
            }
            Ok(())
        });
        tasks.push(handle);
    }

    // すべてのタスクが完了するのを待つ
    for t in tasks {
        let _ = t.await?;
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
