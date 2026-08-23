use crate::constants::MAX_JSON_DOWNLOAD_BYTES;
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::fetch::fetch_json_with_limit;
use crate::ip::{IpFamily, IpSets};
use crate::output::{OutputFormat, write_as_ip_list_to_file};
use futures::{Stream, StreamExt, stream};
use ipnet::IpNet;
use reqwest::Client;
use serde_json::Value;
use std::collections::BTreeSet;
use std::future::Future;
use std::str::FromStr;

type AsProcessOutcome = (u32, Result<(), AppError>);

async fn get_prefixes_via_rdap(
    client: &Client,
    as_number: u32,
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

    Ok(nets.into_iter().collect::<IpSets>().aggregated())
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

async fn fetch_ripe_stat_prefixes(client: &Client, as_number: u32) -> Result<Vec<IpNet>, AppError> {
    let url =
        format!("https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS{as_number}");
    let json: Value = fetch_json_with_limit(client, &url, MAX_JSON_DOWNLOAD_BYTES).await?;
    Ok(extract_prefixes_from_ripe_stat(&json))
}

async fn fetch_arin_originas_prefixes(
    client: &Client,
    as_number: u32,
) -> Result<Vec<IpNet>, AppError> {
    let url =
        format!("https://rdap.arin.net/registry/arin_originas0_networksbyoriginas/{as_number}");
    let json: Value = fetch_json_with_limit(client, &url, MAX_JSON_DOWNLOAD_BYTES).await?;
    Ok(extract_prefixes_from_arin(&json))
}

fn buffered_results<'a, T, Fetch, FetchFuture>(
    as_numbers: &'a [u32],
    concurrency: usize,
    fetch: Fetch,
) -> impl Stream<Item = (u32, Result<T, AppError>)> + 'a
where
    T: 'a,
    Fetch: Fn(u32) -> FetchFuture + Clone + 'a,
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
    as_numbers: &'a [u32],
    concurrency: usize,
    debug: DebugOutput,
) -> impl Stream<Item = (u32, Result<IpSets, AppError>)> + 'a {
    buffered_results(as_numbers, concurrency, move |as_number| {
        get_prefixes_via_rdap(client, as_number, debug)
    })
}

pub(crate) async fn process_as_numbers(
    client: &Client,
    as_numbers: &[u32],
    output_format: OutputFormat,
    concurrency: usize,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let outcomes = fetch_as_results(client, as_numbers, concurrency, debug)
        .then(move |(as_number, result)| async move {
            let result = match result {
                Ok(ip_sets) => write_ip_sets(as_number, &ip_sets, output_format, debug).await,
                Err(error) => Err(error),
            };
            (as_number, result)
        })
        .collect::<Vec<_>>()
        .await;

    outcomes
        .iter()
        .filter_map(|(as_number, result)| result.as_ref().err().map(|error| (as_number, error)))
        .for_each(|(as_number, error)| {
            eprintln!("Warning: failed to process AS{as_number}: {error}");
        });

    finalize_as_processing(outcomes)
}

async fn write_ip_sets(
    as_number: u32,
    ip_sets: &IpSets,
    output_format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    write_ip_list(
        as_number,
        IpFamily::V4,
        ip_sets.ipv4(),
        output_format,
        debug,
    )
    .await?;
    write_ip_list(
        as_number,
        IpFamily::V6,
        ip_sets.ipv6(),
        output_format,
        debug,
    )
    .await
}

async fn write_ip_list(
    as_number: u32,
    family: IpFamily,
    ip_set: &BTreeSet<IpNet>,
    output_format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    if ip_set.is_empty() {
        debug.log(format!("No {} routes for AS{as_number}", family.as_str()));
        Ok(())
    } else {
        write_as_ip_list_to_file(as_number, family, ip_set, output_format, debug).await
    }
}

fn finalize_as_processing(outcomes: Vec<AsProcessOutcome>) -> Result<(), AppError> {
    let failures = outcomes
        .into_iter()
        .filter_map(|(as_number, result)| {
            result.err().map(|error| format!("AS{as_number}: {error}"))
        })
        .collect::<Vec<_>>();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::Other(format!(
            "Failed to process {} AS number(s): {}",
            failures.len(),
            failures.join("; ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        buffered_results, extract_prefixes_from_arin, extract_prefixes_from_ripe_stat,
        finalize_as_processing,
    };
    use crate::error::AppError;
    use futures::StreamExt;
    use serde_json::json;

    #[test]
    fn pure_extractors_ignore_invalid_entries() {
        let ripe = json!({
            "data": {"prefixes": [
                {"prefix": "192.0.2.0/24"},
                {"prefix": "invalid"},
                {"other": "missing"}
            ]}
        });
        let arin = json!({
            "arin_originas0_networkSearchResults": [
                {"ipVersion": "v4", "v4prefix": "198.51.100.0", "length": 24},
                {"ipVersion": "v6", "v6prefix": "2001:db8::", "length": 32},
                {"ipVersion": "v4", "v4prefix": "invalid", "length": 24}
            ]
        });

        assert_eq!(extract_prefixes_from_ripe_stat(&ripe).len(), 1);
        assert_eq!(extract_prefixes_from_arin(&arin).len(), 2);
    }

    #[tokio::test]
    async fn buffered_fetch_preserves_input_order_and_failure() {
        let as_numbers = [3, 1, 2];
        let results = buffered_results(&as_numbers, 2, |as_number| async move {
            if as_number == 1 {
                Err(AppError::Other("failed".into()))
            } else {
                Ok(as_number)
            }
        })
        .collect::<Vec<_>>()
        .await;

        assert_eq!(
            results
                .iter()
                .map(|(as_number, _)| *as_number)
                .collect::<Vec<_>>(),
            as_numbers
        );
        assert!(results.get(1).is_some_and(|(_, result)| result.is_err()));
    }

    #[test]
    fn finalization_reports_partial_failure() {
        let outcomes = vec![
            (1234, Ok::<(), AppError>(())),
            (5678, Err(AppError::Other("boom".into()))),
        ];
        let message = finalize_as_processing(outcomes)
            .err()
            .map(|error| error.to_string());

        assert!(message.as_deref().is_some_and(|text| {
            text.contains("Failed to process 1 AS number(s)") && text.contains("AS5678: boom")
        }));
    }
}
