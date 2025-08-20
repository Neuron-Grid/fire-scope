use crate::constants::RIR_URLS;
use crate::error::AppError;
use crate::fetch::fetch_with_retry;
use futures::future::join_all;
use reqwest::Client;
use crate::common::debug_log;

/// 共通のダウンロード関数。
/// urlsに指定されたURLを並列で全てダウンロードし、
/// 成功したもののテキストと失敗したURLのセットを返す。
pub async fn download_files(
    client: &Client,
    urls: &[&'static str],
    retry_attempts: u32,
    max_backoff_secs: u64,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut handles = Vec::new();

    // tokio::spawnでタスクを生成しながらfetch_with_retry()を呼び出す
    for url in urls {
        let url_owned = url.to_string();
        let client_clone = client.clone();

        let ra = retry_attempts;
        let mx = max_backoff_secs;
        handles.push(tokio::spawn(async move {
            fetch_with_retry(&client_clone, &url_owned, ra, mx).await
        }));
    }

    let results = join_all(handles).await;
    let mut success_texts = Vec::new();
    let mut fail_urls = Vec::new();

    // タスク実行結果を集約
    for (i, res) in results.into_iter().enumerate() {
        match res {
            // タスクは正常終了、内部のfetch処理も成功
            Ok(Ok(text)) => {
                success_texts.push(text);
            }
            // タスクは正常終了したが、内部のfetch処理がエラー
            Ok(Err(e)) => {
                debug_log(format!("HTTP fetch error: {} (url={})", e, urls[i]));
                fail_urls.push(urls[i].to_string());
            }
            // タスク自体が失敗 (パニックなど)
            Err(e) => {
                debug_log(format!("Download task failed: {} (url={})", e, urls[i]));
                fail_urls.push(urls[i].to_string());
            }
        }
    }

    Ok((success_texts, fail_urls))
}

/// RIRファイルのダウンロード関数。
/// 成功テキストと失敗URLのタプルを返す。
pub async fn download_all_rir_files(
    client: &Client,
    retry_attempts: u32,
    max_backoff_secs: u64,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    download_files(client, &RIR_URLS, retry_attempts, max_backoff_secs).await
}
