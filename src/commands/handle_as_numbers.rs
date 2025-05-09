use crate::asn::process_as_numbers;
use crate::common::OutputFormat;
use crate::error::AppError;
use reqwest::Client;

/// ユーザー指定ASリストを受け取りRDAPで処理
pub async fn run_as_numbers(
    client: &Client,
    as_numbers: &[u32],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    // RDAPは純粋な数値のみを期待
    let as_strings: Vec<String> = as_numbers.iter().map(|n| n.to_string()).collect();
    process_as_numbers(client, &as_strings, mode, output_format).await
}
