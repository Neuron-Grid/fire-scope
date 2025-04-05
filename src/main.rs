use clap::Parser;
use fire_scope::cli::Cli;
use reqwest::Client;
use std::error::Error;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // CLI引数を解析
    let args = Cli::parse();
    // 実行
    run(args).await
}

/// CLI引数に応じてライブラリの commands を呼び分ける
async fn run(args: Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    // overlap 指定時
    if args.overlap {
        // fire_scope::commands::handle_overlap::run_overlap(...)
        fire_scope::commands::handle_overlap::run_overlap(&args, &client).await?;
        return Ok(());
    }

    // AS番号指定時
    if let Some(as_list) = &args.as_numbers {
        fire_scope::commands::handle_as_numbers::run_as_numbers(as_list, &args.mode).await?;
        return Ok(());
    }

    // 国コード指定時
    if let Some(country_codes) = &args.country_codes {
        fire_scope::commands::handle_country_codes::run_country_codes(
            country_codes,
            &client,
            &args.mode,
        )
        .await?;
        return Ok(());
    }

    // どれも指定されなかった場合
    eprintln!("Error: Please specify --country or --as-number.\nUse --help for usage.");
    Ok(())
}
