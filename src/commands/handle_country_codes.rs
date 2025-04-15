use crate::common::OutputFormat;
use crate::common_download::download_all_rir_files;
use crate::error::AppError;
use crate::process::process_all_country_codes;
use reqwest::Client;

pub async fn run_country_codes(
    country_codes: &[String],
    client: &Client,
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    // 取得成功したRIRテキストと、失敗URLを受け取る
    let (rir_texts, failed_urls) = download_all_rir_files(client).await?;

    if !failed_urls.is_empty() {
        // 失敗したURLのリストがある場合、ログ出力するなど
        eprintln!("Warning: The following RIR URLs failed to download:");
        for url in &failed_urls {
            eprintln!(" - {}", url);
        }

        // ここで継続するか、エラーとして中断するかを選択できる
        return Err(AppError::Other("Some RIR downloads failed".into()));
    }

    // 成功したrir_textsだけをもとに国コード解析を実施
    process_all_country_codes(country_codes, &rir_texts, mode, output_format).await?;
    Ok(())
}
