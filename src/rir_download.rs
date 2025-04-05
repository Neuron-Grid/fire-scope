use crate::constants::RIR_URLS;
use crate::fetch::fetch_with_retry;
use reqwest::Client;
use std::error::Error;

/// RIRファイルをすべてダウンロードして文字列ベクタとして返す。
pub async fn download_all_rir_files(
    client: &Client,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    use futures::future::join_all;

    let mut handles = Vec::new();
    for url in RIR_URLS {
        let url_owned = url.to_string();
        let client_clone = client.clone();
        // 非同期で複数URLを並列取得
        handles.push(tokio::spawn(async move {
            fetch_with_retry(&client_clone, &url_owned).await
        }));
    }

    let results = join_all(handles).await;
    let mut rir_texts = Vec::new();

    for res in results {
        match res {
            Ok(Ok(text)) => {
                rir_texts.push(text);
            }
            Ok(Err(e)) => {
                eprintln!("HTTP取得エラー: {}", e);
            }
            Err(e) => {
                eprintln!("タスク失敗: {}", e);
            }
        }
    }

    Ok(rir_texts)
}
