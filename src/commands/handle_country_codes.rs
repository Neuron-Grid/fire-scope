use crate::common::OutputFormat;
use crate::error::AppError;
use crate::process::process_all_country_codes;
use crate::rir_download::download_all_rir_files;
use reqwest::Client;

pub async fn run_country_codes(
    country_codes: &[String],
    client: &Client,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let rir_texts = download_all_rir_files(client).await?;
    process_all_country_codes(country_codes, &rir_texts, mode, output_format).await?;
    Ok(())
}
