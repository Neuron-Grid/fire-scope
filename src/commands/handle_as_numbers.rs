use crate::asn::process_as_numbers;
use crate::common::OutputFormat;
use crate::error::AppError;
use reqwest::Client;

/// ユーザーが指定した AS番号リストを受け取り、
/// 内部で `asn::process_as_numbers` を呼び出す。
pub async fn run_as_numbers(
    client: &Client,
    as_numbers: &[u32],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();
    process_as_numbers(client, &as_strings, mode, output_format).await
}
