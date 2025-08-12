use crate::constants::MAX_RIR_DOWNLOAD_BYTES;
use crate::error::AppError;
use futures::StreamExt;
use rand::Rng;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

/// ボディをストリーミングで読み込みつつ、サイズ上限を強制してStringへ変換
async fn read_body_with_limit_to_string(
    resp: reqwest::Response,
    max_bytes: u64,
) -> Result<String, AppError> {
    let mut total: u64 = 0;
    let mut buf: Vec<u8> = Vec::new();

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?; // reqwest::Error -> AppError::Network via ? 上位で変換
        total = total.saturating_add(chunk.len() as u64);
        if total > max_bytes {
            return Err(AppError::Other(format!(
                "Response too large ({} bytes > {} bytes)",
                total, max_bytes
            )));
        }
        buf.extend_from_slice(&chunk);
    }

    let text = String::from_utf8(buf)?; // FromUtf8Error -> AppError::Utf8
    Ok(text)
}

async fn fetch_once(client: &Client, url: &str) -> Result<String, AppError> {
    let resp = client.get(url).send().await?.error_for_status()?; // 非2xxを明示的にエラー化

    if let Some(len) = resp.content_length() {
        if len > MAX_RIR_DOWNLOAD_BYTES {
            return Err(AppError::Other(format!(
                "Response too large ({} bytes > {} bytes): {}",
                len, MAX_RIR_DOWNLOAD_BYTES, url
            )));
        }
    }
    // Content-Length が無い場合にも備えて、常にストリーミングで上限制御
    read_body_with_limit_to_string(resp, MAX_RIR_DOWNLOAD_BYTES).await
}

/// HTTP GETによるデータ取得をリトライ+指数バックオフ付きで行う
/// 失敗時はAppError::Other(...)を返す
pub async fn fetch_with_retry(
    client: &Client,
    url: &str,
    retry_attempts: u32,
    max_backoff_secs: u64,
) -> Result<String, AppError> {
    let attempts = retry_attempts.max(1);
    for i in 0..attempts {
        match fetch_once(client, url).await {
            Ok(text) => {
                return Ok(text);
            }
            Err(e) => {
                eprintln!(
                    "[fetch_with_retry] Error on attempt {}/{}: {}",
                    i + 1,
                    attempts,
                    e
                );
                let sleep_duration = calc_exponential_backoff_duration(i, max_backoff_secs);
                sleep(sleep_duration).await;
            }
        }
    }

    // リトライ失敗
    Err(AppError::Other(format!(
        "Failed to fetch data from {} after {} attempts",
        url, attempts
    )))
}

/// 指数バックオフのスリープ時間を計算するヘルパー関数
fn calc_exponential_backoff_duration(retry_count: u32, max_backoff_secs: u64) -> Duration {
    let mut rng = rand::rng();
    let random_part: f64 = rng.random();

    let base = 2u64.saturating_pow(retry_count);
    let capped = base.min(max_backoff_secs.max(1));
    let backoff_seconds = (capped as f64) + random_part;
    Duration::from_secs_f64(backoff_seconds)
}

/// JSONをサイズ上限制御の上で取得してパース
pub async fn fetch_json_with_limit<T: serde::de::DeserializeOwned>(
    client: &Client,
    url: &str,
    max_bytes: u64,
) -> Result<T, AppError> {
    let resp = client.get(url).send().await?.error_for_status()?;

    if let Some(len) = resp.content_length() {
        if len > max_bytes {
            return Err(AppError::Other(format!(
                "JSON response too large ({} bytes > {} bytes): {}",
                len, max_bytes, url
            )));
        }
    }

    // ボディを上限制御で読み込む
    let text = read_body_with_limit_to_string(resp, max_bytes).await?;
    let value = serde_json::from_str::<T>(&text)
        .map_err(|e| AppError::ParseError(format!("JSON parse error: {e}")))?;
    Ok(value)
}
