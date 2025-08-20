use crate::common::OutputFormat;
use crate::common_download::download_all_rir_files;
use crate::error::AppError;
use crate::process::process_all_country_codes;
use reqwest::Client;
use crate::common::debug_log;

pub async fn run_country_codes(
    country_codes: &[String],
    client: &Client,
    output_format: OutputFormat,
    continue_on_partial: bool,
    retry_attempts: u32,
    max_backoff_secs: u64,
) -> Result<(), AppError> {
    // 取得成功したRIRテキストと、失敗URLを受け取る
    let (rir_texts, failed_urls) =
        download_all_rir_files(client, retry_attempts, max_backoff_secs).await?;

    if !failed_urls.is_empty() {
        // 失敗したURLのリストがある場合、デバッグ時のみ詳細を表示
        debug_log(format!("Some RIR files failed to download: {:?}", failed_urls));
        if !continue_on_partial {
            return Err(AppError::Other(
                "Some RIR downloads failed (use --continue-on-partial to proceed)".into(),
            ));
        }
    }

    if rir_texts.is_empty() {
        return Err(AppError::Other(
            "No RIR files available to process".into(),
        ));
    }

    // 成功したrir_textsだけをもとに国コード解析を実施
    process_all_country_codes(country_codes, &rir_texts, output_format).await?;
    Ok(())
}
