use crate::common::debug_log;
use crate::constants::RIR_URLS;
use crate::error::AppError;
use crate::fetch::fetch_with_retry;
use futures::future::join_all;
use reqwest::Client;

enum DownloadOutcome {
    Success(String),
    Failure { url: String, error: String },
}

fn classify_download(
    url: &str,
    result: Result<Result<String, AppError>, tokio::task::JoinError>,
) -> DownloadOutcome {
    match result {
        Ok(Ok(text)) => DownloadOutcome::Success(text),
        Ok(Err(error)) => DownloadOutcome::Failure {
            url: url.to_string(),
            error: format!("HTTP fetch error: {error}"),
        },
        Err(error) => DownloadOutcome::Failure {
            url: url.to_string(),
            error: format!("Download task failed: {error}"),
        },
    }
}

/// 共通のダウンロード関数。
/// urlsに指定されたURLを並列で全てダウンロードし、
/// 成功したもののテキストと失敗したURLのセットを返す。
pub async fn download_files(
    client: &Client,
    urls: &[&'static str],
    retry_attempts: u32,
    max_backoff_secs: u64,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let handles = urls
        .iter()
        .map(|url| {
            let url = url.to_string();
            let client = client.clone();
            tokio::spawn(async move {
                fetch_with_retry(&client, &url, retry_attempts, max_backoff_secs).await
            })
        })
        .collect::<Vec<_>>();

    let results = join_all(handles).await;
    Ok(urls
        .iter()
        .zip(results)
        .map(|(url, result)| classify_download(url, result))
        .fold((Vec::new(), Vec::new()), |mut downloads, outcome| {
            match outcome {
                DownloadOutcome::Success(text) => downloads.0.push(text),
                DownloadOutcome::Failure { url, error } => {
                    debug_log(format!("{error} (url={url})"));
                    downloads.1.push(url);
                }
            }
            downloads
        }))
}

/// RIRファイルのダウンロード関数。
/// 成功テキストと失敗URLのタプルを返す。
pub async fn download_all_rir_files(
    client: &Client,
    retry_attempts: u32,
    max_backoff_secs: u64,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    download_files(client, RIR_URLS, retry_attempts, max_backoff_secs).await
}

#[cfg(test)]
mod tests {
    use super::{DownloadOutcome, classify_download};
    use crate::error::AppError;

    #[test]
    fn classifies_download_results_without_network_access() {
        assert!(matches!(
            classify_download("https://example.test", Ok(Ok("body".into()))),
            DownloadOutcome::Success(text) if text == "body"
        ));
        assert!(matches!(
            classify_download(
                "https://example.test",
                Ok(Err(AppError::Other("failed".into())))
            ),
            DownloadOutcome::Failure { url, error }
                if url == "https://example.test" && error.contains("failed")
        ));
    }
}
