use crate::asn::process_as_numbers;
use crate::common::OutputFormat;
use crate::error::AppError;

pub async fn run_as_numbers(
    as_numbers: &[u32],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), AppError> {
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();
    process_as_numbers(&as_strings, mode, output_format).await
}
