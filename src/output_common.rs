use crate::error::AppError;
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

pub(crate) fn sanitize_identifier(input: &str) -> String {
    let sanitized = input
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let truncated = sanitized
        .trim_matches('_')
        .chars()
        .take(64)
        .collect::<String>();

    if truncated.is_empty() {
        "UNKNOWN".to_owned()
    } else {
        truncated
    }
}

pub(crate) fn make_header(now: &str, country_code: &str, as_number: &str) -> String {
    format!("# Generated at: {now}\n# Country Code: {country_code}\n# AS Number: {as_number}\n\n")
}

pub(crate) async fn write_list_txt(
    path: &Path,
    ipnets: &BTreeSet<IpNet>,
    header: &str,
) -> Result<(), AppError> {
    atomic_write(path, render_txt(ipnets, header).as_bytes()).await
}

fn render_txt(ipnets: &BTreeSet<IpNet>, header: &str) -> String {
    let body = ipnets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    format!("{header}{body}\n")
}

pub(crate) async fn write_list_nft(
    path: &Path,
    ipnets: &BTreeSet<IpNet>,
    header: &str,
) -> Result<(), AppError> {
    let define_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .map_or_else(|| "unknown_define".to_owned(), sanitize_identifier);
    atomic_write(path, render_nft(ipnets, header, &define_name).as_bytes()).await
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

async fn atomic_write(path: &Path, content: &[u8]) -> Result<(), AppError> {
    let directory = path
        .parent()
        .map_or_else(|| Path::new("."), |parent| parent);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map_or("output", |name| name);
    let mut temporary_path = PathBuf::from(directory);
    temporary_path.push(format!(".{file_name}.tmp.{}", rand::random::<u64>()));

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary_path)
        .await?;

    let write_result = async {
        file.write_all(content).await?;
        file.sync_all().await
    }
    .await;
    drop(file);

    if let Err(error) = write_result {
        let _cleanup_result = fs::remove_file(&temporary_path).await;
        return Err(error.into());
    }

    if let Err(error) = fs::rename(&temporary_path, path).await {
        let _cleanup_result = fs::remove_file(&temporary_path).await;
        return Err(error.into());
    }

    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/output_common.rs"]
mod tests;
