use rand::Rng;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

/// HTTP GETを1回だけ実行する。
/// 成功時はレスポンスボディを文字列として返す。
/// 失敗時はエラーを返す。
async fn fetch_once(
    client: &Client,
    url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client.get(url).send().await?;
    let text = resp.text().await?;
    Ok(text)
}

/// HTTP GETによるデータ取得を、リトライ+指数バックオフ付きで行う。
/// 成功時はレスポンス文字列を返す。
/// retry_attempts回失敗した場合、エラーを返す。
pub async fn fetch_with_retry(
    client: &Client,
    url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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

    Err(format!(
        "Failed to fetch data from {} after {} attempts.",
        url, retry_attempts
    )
    .into())
}

/// 指数バックオフのスリープ時間を計算するヘルパー関数
fn calc_exponential_backoff_duration(retry_count: u32) -> Duration {
    let mut rng = rand::rng();
    let random_part: f64 = rng.random();

    let base = 2u64.pow(retry_count);
    let backoff_seconds = (base as f64) + random_part;
    Duration::from_secs_f64(backoff_seconds)
}
