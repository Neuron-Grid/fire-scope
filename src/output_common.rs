use ipnet::IpNet;
use std::{collections::BTreeSet, path::Path};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

/// 汎用ヘッダー生成: こちらは非同期要素がないのでそのまま
pub fn make_header(now_str: &str, country_code: &str, as_number: &str) -> String {
    format!(
        "# Generated at: {}\n# Country Code: {}\n# AS Number: {}\n\n",
        now_str, country_code, as_number
    )
}

/// TXT出力用の共通ヘルパー
pub async fn write_list_txt<P: AsRef<Path>>(
    path: P,
    ipnets: &BTreeSet<IpNet>,
    mode: &str,
    header: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = ipnets
        .iter()
        .map(|net| net.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!("{}{}\n", header, body);

    match mode {
        "append" => {
            // 非同期OpenOptions
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await?;
            file.write_all(content.as_bytes()).await?;
        }
        _ => {
            // まるごと書き込む場合
            fs::write(path, &content).await?;
        }
    }

    Ok(())
}

/// NFT 出力用の共通ヘルパー
pub async fn write_list_nft<P: AsRef<Path>>(
    path: P,
    ipnets: &BTreeSet<IpNet>,
    mode: &str,
    header: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file_path = path.as_ref();
    let define_name = file_path
        .file_stem()
        .and_then(|os| os.to_str())
        .unwrap_or("unknown_define");

    let mut content = String::new();
    content.push_str(header);
    content.push_str(&format!("define {} {{\n", define_name));
    for net in ipnets {
        content.push_str(&format!("    {},\n", net));
    }
    content.push_str("}\n");

    match mode {
        "append" => {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)
                .await?;
            file.write_all(content.as_bytes()).await?;
        }
        _ => {
            fs::write(file_path, &content).await?;
        }
    }

    Ok(())
}
