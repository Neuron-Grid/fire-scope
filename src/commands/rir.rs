use crate::cli::RirOptions;
use crate::country::CountryMap;
use crate::diagnostics::{DebugOutput, write_stderr};
use crate::error::AppError;
use crate::parse::parse_all_country_codes;
use crate::rir_download::download_all_rir_files;
use reqwest::Client;

pub(super) async fn load_country_map(
    client: &Client,
    country_codes: &[String],
    options: RirOptions,
    debug: DebugOutput,
) -> Result<CountryMap, AppError> {
    let (texts, failures) =
        download_all_rir_files(client, options.attempts, options.max_backoff_secs, debug).await;

    for (url, error) in &failures {
        debug.log(format!("HTTP fetch error: {error} (url={url})"));
    }
    if !failures.is_empty() {
        let failed_urls = failures
            .iter()
            .map(|(url, _)| url.as_str())
            .collect::<Vec<_>>();
        write_stderr(format_args!(
            "Warning: {} RIR download(s) failed: {}",
            failures.len(),
            failed_urls.join(", ")
        ));
    }

    let texts = apply_rir_failure_policy(texts, failures.len(), options.continue_on_partial)?;
    let selected = country_codes.to_owned();
    tokio::task::spawn_blocking(move || parse_all_country_codes(&texts, &selected)).await?
}

fn apply_rir_failure_policy(
    texts: Vec<String>,
    failed_count: usize,
    continue_on_partial: bool,
) -> Result<Vec<String>, AppError> {
    if failed_count > 0 && !continue_on_partial {
        return Err(AppError::Other(
            "Some RIR downloads failed (use --continue-on-partial to proceed)".into(),
        ));
    }

    (!texts.is_empty())
        .then_some(texts)
        .ok_or_else(|| AppError::Other("No RIR files available to process".into()))
}

#[cfg(test)]
#[path = "../../tests/unit/commands_rir.rs"]
mod tests;
