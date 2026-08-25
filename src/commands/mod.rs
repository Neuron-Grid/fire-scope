mod list_asn;
mod list_country;
mod overlap;
mod rir;

use crate::cli::{Cli, Command, ListCommand};
use crate::diagnostics::{DebugOutput, write_stderr};
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
            write_stderr(format_args!("Error: {error}"));
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
