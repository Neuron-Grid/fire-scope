use crate::asn::fetch_as_results;
use crate::cli::AsnArgs;
use crate::diagnostics::{DebugOutput, write_stderr};
use crate::error::AppError;
use crate::output::{OutputFormat, write_as_ip_lists_to_files};
use futures::StreamExt;
use reqwest::Client;
use std::num::NonZeroU32;

type AsProcessOutcome = (NonZeroU32, Result<(), AppError>);

pub(super) async fn run(
    client: &Client,
    args: AsnArgs,
    output_format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let outcomes = fetch_as_results(client, &args.as_numbers, args.query.concurrency, debug)
        .then(move |(as_number, result)| async move {
            let result = match result {
                Ok(ip_sets) => {
                    write_as_ip_lists_to_files(as_number, &ip_sets, output_format, debug).await
                }
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
            write_stderr(format_args!(
                "Warning: failed to process AS{as_number}: {error}"
            ));
        });

    finalize_as_processing(outcomes)
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
#[path = "../../tests/unit/commands_list_asn.rs"]
mod tests;
