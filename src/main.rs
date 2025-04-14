use clap::Parser;
use fire_scope::cli::Cli;
use fire_scope::common::OutputFormat;
use fire_scope::error::AppError;
use reqwest::Client;
use std::str::FromStr;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), AppError> {
    let args = Cli::parse();
    run(args).await
}

async fn run(args: Cli) -> Result<(), AppError> {
    let client = Client::new();

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
        fire_scope::commands::handle_overlap::run_overlap(&args, &client, format_enum).await?;
        return Ok(());
    }

    if let Some(as_list) = &args.as_numbers {
        fire_scope::commands::handle_as_numbers::run_as_numbers(as_list, &args.mode, format_enum)
            .await?;
        return Ok(());
    }

    if let Some(country_codes) = &args.country_codes {
        fire_scope::commands::handle_country_codes::run_country_codes(
            country_codes,
            &client,
            &args.mode,
            format_enum,
        )
        .await?;
        return Ok(());
    }

    eprintln!("Error: Please specify --country or --as-number.\nUse --help for usage.");
    Ok(())
}
