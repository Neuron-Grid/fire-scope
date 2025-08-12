use crate::constants::MAX_RIR_DOWNLOAD_BYTES;
use crate::error::AppError;
use rand::Rng;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

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
    let text = resp.text().await?;
    Ok(text)
}

/// HTTP GETによるデータ取得をリトライ+指数バックオフ付きで行う
/// 失敗時はAppError::Other(...)を返す
pub async fn fetch_with_retry(client: &Client, url: &str) -> Result<String, AppError> {
    let retry_attempts = 10;

    for i in 0..retry_attempts {
        match fetch_once(client, url).await {
            Ok(text) => {
                return Ok(text);
            }
            Err(e) => {
                eprintln!(
                    "[fetch_with_retry] Error on attempt {}/{}: {}",
                    i + 1,
                    retry_attempts,
                    e
                );
                let sleep_duration = calc_exponential_backoff_duration(i);
                sleep(sleep_duration).await;
            }
        }
    }

    // リトライ失敗
    Err(AppError::Other(format!(
        "Failed to fetch data from {} after {} attempts",
        url, retry_attempts
    )))
}

/// 指数バックオフのスリープ時間を計算するヘルパー関数
fn calc_exponential_backoff_duration(retry_count: u32) -> Duration {
    let mut rng = rand::rng();
    let random_part: f64 = rng.random();

    let base = 2u64.pow(retry_count);
    let backoff_seconds = (base as f64) + random_part;
    Duration::from_secs_f64(backoff_seconds)
}
