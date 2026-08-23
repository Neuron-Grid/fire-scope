use crate::error::AppError;
use ipnet::IpNet;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

/// 出力用の安全な識別子に正規化する
/// - 非ASCII英数字はアンダースコアに置換
/// - 先頭末尾のアンダースコアは削除
/// - 長すぎる場合は64文字に切り詰め
/// - 空になった場合は "UNKNOWN"
///
/// # Examples
///
/// ```
/// use fire_scope::output_common::sanitize_identifier;
///
/// assert_eq!(sanitize_identifier("JP"), "JP");
/// assert_eq!(sanitize_identifier("hello world"), "hello_world");
/// assert_eq!(sanitize_identifier("---"), "UNKNOWN");
/// ```
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
    if s.is_empty() {
        "UNKNOWN".to_string()
    } else {
        s
    }
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
    let content = render_txt(ipnets, header);
    atomic_write(path.as_ref(), content.as_bytes()).await
}

fn render_txt(ipnets: &BTreeSet<IpNet>, header: &str) -> String {
    let body = ipnets
        .iter()
        .map(|net| net.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    format!("{header}{body}\n")
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

    let content = render_nft(ipnets, header, &define_name);
    atomic_write(file_path, content.as_bytes()).await
}

fn render_nft(ipnets: &BTreeSet<IpNet>, header: &str, define_name: &str) -> String {
    let entries = ipnets
        .iter()
        .map(|net| format!("    {net}"))
        .collect::<Vec<_>>()
        .join(",\n");
    let body = if entries.is_empty() {
        String::new()
    } else {
        format!("{entries}\n")
    };
    format!("{header}define {define_name} = {{\n{body}}}\n")
}

/// 一時ファイルに書いてから `rename` で置換する。
/// `rename` に失敗した場合は既存ファイルを保持したままエラーを返す。
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

    // 同一ディレクトリ内の rename による置換を試みる。
    // 失敗時は既存ファイルを削除せず、一時ファイルのみ best-effort で掃除する。
    match fs::rename(&tmp_path, path).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp_path).await;
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{render_nft, render_txt};
    use ipnet::IpNet;
    use std::{collections::BTreeSet, str::FromStr};

    fn sample_ipnets() -> BTreeSet<IpNet> {
        ["192.0.2.0/24", "2001:db8::/32"]
            .into_iter()
            .filter_map(|cidr| IpNet::from_str(cidr).ok())
            .collect()
    }

    #[test]
    fn renderers_are_deterministic() {
        let ipnets = sample_ipnets();

        assert_eq!(
            render_txt(&ipnets, "# header\n"),
            "# header\n192.0.2.0/24\n2001:db8::/32\n"
        );
        assert_eq!(
            render_nft(&ipnets, "# header\n", "routes"),
            "# header\ndefine routes = {\n    192.0.2.0/24,\n    2001:db8::/32\n}\n"
        );
    }
}
