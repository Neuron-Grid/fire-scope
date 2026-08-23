use crate::asn::process_as_numbers;
use crate::cli::AsnArgs;
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::output::OutputFormat;
use reqwest::Client;

pub(super) async fn run(
    client: &Client,
    args: AsnArgs,
    output_format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    process_as_numbers(
        client,
        &args.as_numbers,
        output_format,
        args.query.concurrency,
        debug,
    )
    .await
}
