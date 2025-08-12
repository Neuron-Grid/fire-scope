use crate::error::AppError;
use ipnet::IpNet;
use std::{collections::BTreeSet, path::{Path, PathBuf}};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

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

    // 常に上書き（原子的に安全な書き込み）
    atomic_write(path.as_ref(), content.as_bytes()).await?;

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

    // 常に上書き（原子的に安全な書き込み）
    atomic_write(file_path, content.as_bytes()).await?;

    Ok(())
}

/// 一時ファイルに書いてから原子的に`rename`で置換する安全な書き込み
async fn atomic_write(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp_path = PathBuf::from(dir);
    let fname = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let suffix: u64 = rand::random();
    tmp_path.push(format!(".{}.tmp.{}", fname, suffix));

    // 作成（既存不可）→ 書き込み → fsync
    {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp_path)
            .await?;
        file.write_all(content).await?;
        // データの同期（失敗はそのままエラー伝播）
        file.sync_all().await?;
    }

    // 原子的置換（Unixは既存を置換、Windowsは失敗しうるためフォールバック）
    match fs::rename(&tmp_path, path).await {
        Ok(_) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(path).await;
            fs::rename(&tmp_path, path).await?;
            Ok(())
        }
    }
}
