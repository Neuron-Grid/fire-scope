use crate::asn::process_as_numbers;
use std::error::Error;

/// AS番号リストを受け取り、文字列("ASxxxx") へ変換して実行するラッパ関数。
pub async fn run_as_numbers(
    as_numbers: &[u32],
    mode: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // u32 -> "ASxxxx" 形式の文字列へ
    let as_strings: Vec<String> = as_numbers.iter().map(|n| format!("AS{}", n)).collect();

    // asn.rs 内の関数を呼び出す
    process_as_numbers(&as_strings, mode).await?;
    Ok(())
}
