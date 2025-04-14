use crate::constants::RIR_URLS;
use crate::error::AppError;
use crate::fetch::fetch_with_retry;
use futures::future::join_all;
use reqwest::Client;

/// RIRファイルをすべてダウンロードし、
/// (成功したテキストのVec, 失敗したURLのVec) を返す。
pub async fn download_all_rir_files(
    client: &Client,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut handles = Vec::new();
    let mut urls = Vec::new();

    for url in RIR_URLS {
        let url_owned = url.to_string();
        let client_clone = client.clone();
        urls.push(url_owned.clone());

        // fetch_with_retry() をtokio::spawnで並列実行
        handles.push(tokio::spawn(async move {
            fetch_with_retry(&client_clone, &url_owned).await
        }));
    }

    let results = join_all(handles).await;
    let mut success_texts = Vec::new();
    let mut fail_urls = Vec::new();

    // 各タスクの結果をまとめる
    for (i, res) in results.into_iter().enumerate() {
        match res {
            Ok(Ok(text)) => {
                // 取得成功
                success_texts.push(text);
            }
            Ok(Err(e)) => {
                // HTTPやパースなどのエラー
                eprintln!("HTTP取得エラー: {} (URL={})", e, urls[i]);
                fail_urls.push(urls[i].clone());
            }
            Err(e) => {
                // タスク自体が失敗
                eprintln!("タスク失敗: {} (URL={})", e, urls[i]);
                fail_urls.push(urls[i].clone());
            }
        }
    }

    // 成功・失敗をまとめて呼び出し元に返す
    Ok((success_texts, fail_urls))
}
