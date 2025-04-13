use crate::asn::process_as_numbers;
use crate::common::OutputFormat;
use std::error::Error;

/// AS番号リストを受け取り、文字列("ASxxxx") へ変換して実行するラッパ関数。
pub async fn run_as_numbers(
    as_numbers: &[u32],
    mode: &str,
    output_format: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // u32 -> "ASxxxx" 形式の文字列へ
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();

    // asn.rs内の非同期関数を呼ぶ
    process_as_numbers(&as_strings, mode, output_format).await?;
    Ok(())
}
