use clap::Parser;
use fire_scope::cli::Cli;
use fire_scope::common::OutputFormat;
use fire_scope::error::AppError;
use std::str::FromStr;
use std::time::Duration;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), AppError> {
    let args = Cli::parse();
    run(args).await
}

async fn run(args: Cli) -> Result<(), AppError> {
    // HTTPクライアント（タイムアウト付き）
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(args.http_timeout_secs))
        .connect_timeout(Duration::from_secs(args.connect_timeout_secs))
        .tcp_keepalive(Duration::from_secs(30))
        .user_agent(format!("fire-scope/{} (+https://github.com/Neuron-Grid/fire-scope)", env!("CARGO_PKG_VERSION")))
        .build()?;

    let format_enum = match OutputFormat::from_str(&args.output_format) {
        Ok(fmt) => fmt,
        Err(_) => {
            eprintln!(
                "Warning: Invalid output format '{}'. Using default 'txt'.",
                args.output_format
            );
            OutputFormat::Txt
        }
    };

    if args.overlap {
        // Overlap mode
        fire_scope::commands::handle_overlap::run_overlap(
            &args,
            &client,
            format_enum,
        ).await?;
        return Ok(());
    }

    if let Some(as_list) = &args.as_numbers {
        // AS番号指定時
        fire_scope::commands::handle_as_numbers::run_as_numbers(
            &client,
            as_list,
            format_enum,
            args.concurrency,
        )
        .await?;
        return Ok(());
    }

    if let Some(country_codes) = &args.country_codes {
        // 国コード指定時
        fire_scope::commands::handle_country_codes::run_country_codes(
            country_codes,
            &client,
            format_enum,
            args.continue_on_partial,
            args.max_retries,
            args.max_backoff_sec,
        )
        .await?;
        return Ok(());
    }

    eprintln!("Error: Please specify --country or --as-number.\nUse --help for usage.");
    Err(AppError::InvalidInput(
        "Either --country or --as-number must be specified".into(),
    ))
}
