use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::fetch::fetch_with_retry;
use futures::future::join_all;
use reqwest::Client;
use std::num::{NonZeroU32, NonZeroU64};

const RIR_URLS: &[&str] = &[
    "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest",
    "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest",
    "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest",
    "https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest",
    "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest",
];

fn partition_downloads(
    urls: &[&str],
    results: Vec<Result<String, AppError>>,
) -> (Vec<String>, Vec<(String, AppError)>) {
    urls.iter()
        .zip(results)
        .fold((Vec::new(), Vec::new()), |mut downloads, (url, result)| {
            match result {
                Ok(text) => downloads.0.push(text),
                Err(error) => downloads.1.push(((*url).to_owned(), error)),
            }
            downloads
        })
}

pub(crate) async fn download_all_rir_files(
    client: &Client,
    retry_attempts: NonZeroU32,
    max_backoff_secs: NonZeroU64,
    debug: DebugOutput,
) -> (Vec<String>, Vec<(String, AppError)>) {
    let results = join_all(
        RIR_URLS
            .iter()
            .map(|url| fetch_with_retry(client, url, retry_attempts, max_backoff_secs, debug)),
    )
    .await;

    partition_downloads(RIR_URLS, results)
}

#[cfg(test)]
#[path = "../tests/unit/rir_download.rs"]
mod tests;
