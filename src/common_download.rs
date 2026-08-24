use crate::constants::RIR_URLS;
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::fetch::fetch_with_retry;
use futures::future::join_all;
use reqwest::Client;
use std::num::{NonZeroU32, NonZeroU64};

enum DownloadOutcome {
    Success(String),
    Failure { url: String, error: String },
}

fn classify_download(url: &str, result: Result<String, AppError>) -> DownloadOutcome {
    match result {
        Ok(text) => DownloadOutcome::Success(text),
        Err(error) => DownloadOutcome::Failure {
            url: url.to_owned(),
            error: format!("HTTP fetch error: {error}"),
        },
    }
}

async fn download_files(
    client: &Client,
    urls: &[&str],
    retry_attempts: NonZeroU32,
    max_backoff_secs: NonZeroU64,
    debug: DebugOutput,
) -> (Vec<String>, Vec<String>) {
    let results = join_all(
        urls.iter()
            .map(|url| fetch_with_retry(client, url, retry_attempts, max_backoff_secs, debug)),
    )
    .await;

    urls.iter()
        .zip(results)
        .map(|(url, result)| classify_download(url, result))
        .fold((Vec::new(), Vec::new()), |mut downloads, outcome| {
            match outcome {
                DownloadOutcome::Success(text) => downloads.0.push(text),
                DownloadOutcome::Failure { url, error } => {
                    debug.log(format!("{error} (url={url})"));
                    downloads.1.push(url);
                }
            }
            downloads
        })
}

pub(crate) async fn download_all_rir_files(
    client: &Client,
    retry_attempts: NonZeroU32,
    max_backoff_secs: NonZeroU64,
    debug: DebugOutput,
) -> (Vec<String>, Vec<String>) {
    download_files(client, RIR_URLS, retry_attempts, max_backoff_secs, debug).await
}

#[cfg(test)]
#[path = "../tests/unit/common_download.rs"]
mod tests;
