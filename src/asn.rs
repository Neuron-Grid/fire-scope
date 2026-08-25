use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::fetch::fetch_json_with_limit;
use crate::ip::IpSets;
use futures::{Stream, StreamExt, stream};
use ipnet::IpNet;
use reqwest::Client;
use serde_json::Value;
use std::future::Future;
use std::num::NonZeroU32;
use std::str::FromStr;

const MAX_JSON_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024;

async fn fetch_as_prefixes(
    client: &Client,
    as_number: NonZeroU32,
    debug: DebugOutput,
) -> Result<IpSets, AppError> {
    let nets = match fetch_ripe_stat_prefixes(client, as_number).await {
        Ok(ripe) => {
            let arin = match fetch_arin_originas_prefixes(client, as_number).await {
                Ok(prefixes) => prefixes,
                Err(error) => {
                    debug.log(format!(
                        "ARIN best-effort fetch failed for AS{as_number}: {error}"
                    ));
                    Vec::new()
                }
            };
            ripe.into_iter().chain(arin).collect::<Vec<_>>()
        }
        Err(error) => {
            debug.log(format!("RIPEstat fetch failed for AS{as_number}: {error}"));
            fetch_arin_originas_prefixes(client, as_number).await?
        }
    };

    tokio::task::spawn_blocking(move || nets.into_iter().collect::<IpSets>().aggregated())
        .await
        .map_err(AppError::from)
}

fn extract_prefixes_from_arin(value: &Value) -> Vec<IpNet> {
    value
        .get("arin_originas0_networkSearchResults")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|object| {
            let (prefix_key, length_key) = match object.get("ipVersion").and_then(Value::as_str) {
                Some("v4") => ("v4prefix", "length"),
                Some("v6") => ("v6prefix", "length"),
                _ => return None,
            };
            object
                .get(prefix_key)
                .and_then(Value::as_str)
                .zip(object.get(length_key).and_then(Value::as_u64))
                .and_then(|(prefix, length)| IpNet::from_str(&format!("{prefix}/{length}")).ok())
        })
        .collect()
}

fn extract_prefixes_from_ripe_stat(value: &Value) -> Vec<IpNet> {
    value
        .get("data")
        .and_then(|data| data.get("prefixes"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|object| {
            object
                .get("prefix")
                .and_then(Value::as_str)
                .and_then(|prefix| IpNet::from_str(prefix).ok())
        })
        .collect()
}

async fn fetch_ripe_stat_prefixes(
    client: &Client,
    as_number: NonZeroU32,
) -> Result<Vec<IpNet>, AppError> {
    let url =
        format!("https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS{as_number}");
    let json: Value = fetch_json_with_limit(client, &url, MAX_JSON_DOWNLOAD_BYTES).await?;
    tokio::task::spawn_blocking(move || extract_prefixes_from_ripe_stat(&json))
        .await
        .map_err(AppError::from)
}

async fn fetch_arin_originas_prefixes(
    client: &Client,
    as_number: NonZeroU32,
) -> Result<Vec<IpNet>, AppError> {
    let url =
        format!("https://rdap.arin.net/registry/arin_originas0_networksbyoriginas/{as_number}");
    let json: Value = fetch_json_with_limit(client, &url, MAX_JSON_DOWNLOAD_BYTES).await?;
    tokio::task::spawn_blocking(move || extract_prefixes_from_arin(&json))
        .await
        .map_err(AppError::from)
}

fn buffered_results<'a, T, Fetch, FetchFuture>(
    as_numbers: &'a [NonZeroU32],
    concurrency: usize,
    fetch: Fetch,
) -> impl Stream<Item = (NonZeroU32, Result<T, AppError>)> + 'a
where
    T: 'a,
    Fetch: Fn(NonZeroU32) -> FetchFuture + Clone + 'a,
    FetchFuture: Future<Output = Result<T, AppError>> + 'a,
{
    stream::iter(as_numbers.iter().copied())
        .map(move |as_number| {
            let fetch = fetch.clone();
            async move { (as_number, fetch(as_number).await) }
        })
        .buffered(concurrency.max(1))
}

pub(crate) fn fetch_as_results<'a>(
    client: &'a Client,
    as_numbers: &'a [NonZeroU32],
    concurrency: usize,
    debug: DebugOutput,
) -> impl Stream<Item = (NonZeroU32, Result<IpSets, AppError>)> + 'a {
    buffered_results(as_numbers, concurrency, move |as_number| {
        fetch_as_prefixes(client, as_number, debug)
    })
}

#[cfg(test)]
#[path = "../tests/unit/asn.rs"]
mod tests;
