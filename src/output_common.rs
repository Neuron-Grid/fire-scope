use crate::error::AppError;
use ipnet::IpNet;
use std::{collections::BTreeSet, path::Path};
use tokio::fs::{self};

/// 出力用の安全な識別子に正規化する
/// - 非ASCII英数字はアンダースコアに置換
/// - 先頭末尾のアンダースコアは削除
/// - 長すぎる場合は64文字に切り詰め
/// - 空になった場合は "UNKNOWN"
pub fn sanitize_identifier(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            s.push(ch);
        } else {
            s.push('_');
        }
    }

    // 先頭末尾のアンダースコア除去
    let s = s.trim_matches('_').to_string();
    let s = if s.len() > 64 { s[..64].to_string() } else { s };
    if s.is_empty() { "UNKNOWN".to_string() } else { s }
}

/// 汎用ヘッダー生成
pub fn make_header(now_str: &str, country_code: &str, as_number: &str) -> String {
    format!(
        "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
        now_str, country_code, as_number
    )
}

pub async fn write_list_txt<P: AsRef<Path>>(
    path: P,
    ipnets: &BTreeSet<IpNet>,
    header: &str,
) -> Result<(), AppError> {
    let body = ipnets
        .iter()
        .map(|net| net.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!("{}{}\n", header, body);

    // 常に上書き
    fs::write(path, &content).await?; // io::Error -> AppError::Io

    Ok(())
}

pub async fn write_list_nft<P: AsRef<Path>>(
    path: P,
    ipnets: &BTreeSet<IpNet>,
    header: &str,
) -> Result<(), AppError> {
    let file_path = path.as_ref();
    let define_name_raw = file_path
        .file_stem()
        .and_then(|os| os.to_str())
        .unwrap_or("unknown_define");
    let define_name = sanitize_identifier(define_name_raw);

    let mut content = String::new();
    content.push_str(header);
    content.push_str(&format!("define {} = {{\n", define_name));
    for net in ipnets {
        content.push_str(&format!("    {},\n", net));
    }
    content.push_str("}\n");

    // 常に上書き
    fs::write(file_path, &content).await?;

    Ok(())
}
