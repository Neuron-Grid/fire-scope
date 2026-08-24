mod list_asn;
mod list_country;
mod overlap;

use crate::cli::{Cli, Command, ListCommand, RirOptions};
use crate::common_download::download_all_rir_files;
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use clap::Parser;
use reqwest::Client;
use std::process::ExitCode;
use std::time::Duration;

/// Parses the process arguments, runs the selected command, and returns its exit status.
pub async fn run() -> ExitCode {
    match execute(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn execute(cli: Cli) -> Result<(), AppError> {
    let client = build_client(&cli)?;
    let output_format = cli.output_format;
    let debug = DebugOutput::new(cli.debug);

    match cli.command {
        Command::List {
            target: ListCommand::Country(args),
        } => list_country::run(&client, args, output_format, debug).await,
        Command::List {
            target: ListCommand::Asn(args),
        } => list_asn::run(&client, args, output_format, debug).await,
        Command::Overlap(args) => overlap::run(&client, args, output_format, debug).await,
    }
}

fn build_client(cli: &Cli) -> Result<Client, AppError> {
    Client::builder()
        .timeout(Duration::from_secs(cli.http_timeout_secs.get()))
        .connect_timeout(Duration::from_secs(cli.connect_timeout_secs.get()))
        .tcp_keepalive(Duration::from_secs(30))
        .user_agent(format!(
            "fire-scope/{} (+https://github.com/Neuron-Grid/fire-scope)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(AppError::from)
}

pub(super) async fn download_rir_texts(
    client: &Client,
    options: RirOptions,
    debug: DebugOutput,
) -> Result<Vec<String>, AppError> {
    let (texts, failed_urls) =
        download_all_rir_files(client, options.attempts, options.max_backoff_secs, debug).await;

    if !failed_urls.is_empty() {
        eprintln!(
            "Warning: {} RIR download(s) failed: {}",
            failed_urls.len(),
            failed_urls.join(", ")
        );
    }

    apply_rir_failure_policy(texts, failed_urls.len(), options.continue_on_partial)
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
#[path = "../../tests/unit/commands.rs"]
mod tests;
