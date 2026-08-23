use crate::constants::MAX_RIR_DOWNLOAD_BYTES;
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use futures::StreamExt;
use reqwest::Client;
use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;
use tokio::time::sleep;

async fn read_body_with_limit_to_string(
    response: reqwest::Response,
    max_bytes: u64,
) -> Result<String, AppError> {
    let mut total = 0u64;
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_len = u64::try_from(chunk.len())
            .map_err(|_| AppError::Other("Response chunk length exceeds u64".into()))?;
        total = total
            .checked_add(chunk_len)
            .ok_or_else(|| AppError::Other("Response size overflow".into()))?;
        if total > max_bytes {
            return Err(AppError::Other(format!(
                "Response too large ({total} bytes > {max_bytes} bytes)"
            )));
        }
        buffer.extend_from_slice(&chunk);
    }

    String::from_utf8(buffer).map_err(AppError::from)
}

async fn fetch_once(client: &Client, url: &str) -> Result<String, AppError> {
    let response = client.get(url).send().await?.error_for_status()?;

    if response
        .content_length()
        .is_some_and(|length| length > MAX_RIR_DOWNLOAD_BYTES)
    {
        return Err(AppError::Other(format!(
            "Response too large (> {MAX_RIR_DOWNLOAD_BYTES} bytes): {url}"
        )));
    }

    read_body_with_limit_to_string(response, MAX_RIR_DOWNLOAD_BYTES).await
}

pub(crate) async fn fetch_with_retry(
    client: &Client,
    url: &str,
    retry_attempts: NonZeroU32,
    max_backoff_secs: NonZeroU64,
    debug: DebugOutput,
) -> Result<String, AppError> {
    let attempts = retry_attempts.get();
    for retry_count in 0..attempts {
        match fetch_once(client, url).await {
            Ok(text) => return Ok(text),
            Err(error) => {
                debug.log(format!(
                    "fetch attempt {}/{} failed: {}",
                    retry_count.saturating_add(1),
                    attempts,
                    error
                ));
                if retry_count.saturating_add(1) < attempts {
                    let jitter_fraction = rand::random::<f64>();
                    sleep(exponential_backoff_duration(
                        retry_count,
                        max_backoff_secs,
                        jitter_fraction,
                    ))
                    .await;
                }
            }
        }
    }

    Err(AppError::Other(format!(
        "Failed to fetch data from {url} after {attempts} attempts"
    )))
}

fn exponential_backoff_duration(
    retry_count: u32,
    max_backoff_secs: NonZeroU64,
    jitter_fraction: f64,
) -> Duration {
    let range_secs = 2u64.saturating_pow(retry_count).min(max_backoff_secs.get());
    let jitter = if jitter_fraction.is_finite() {
        jitter_fraction.clamp(0.0, 1.0)
    } else {
        0.0
    };
    Duration::from_secs(range_secs).mul_f64(jitter)
}

pub(crate) async fn fetch_json_with_limit<T: serde::de::DeserializeOwned>(
    client: &Client,
    url: &str,
    max_bytes: u64,
) -> Result<T, AppError> {
    let response = client.get(url).send().await?.error_for_status()?;

    if response
        .content_length()
        .is_some_and(|length| length > max_bytes)
    {
        return Err(AppError::Other(format!(
            "JSON response too large (> {max_bytes} bytes): {url}"
        )));
    }

    let text = read_body_with_limit_to_string(response, max_bytes).await?;
    serde_json::from_str(&text)
        .map_err(|error| AppError::ParseError(format!("JSON parse error: {error}")))
}

#[cfg(test)]
mod tests {
    use super::exponential_backoff_duration;
    use crate::error::AppError;
    use std::num::NonZeroU64;
    use std::time::Duration;

    #[test]
    fn backoff_is_deterministic_for_injected_jitter() -> Result<(), AppError> {
        let cap = NonZeroU64::new(16)
            .ok_or_else(|| AppError::Other("non-zero test cap is invalid".into()))?;

        assert_eq!(exponential_backoff_duration(3, cap, 0.0), Duration::ZERO);
        assert_eq!(
            exponential_backoff_duration(3, cap, 0.5),
            Duration::from_secs(4)
        );
        assert_eq!(
            exponential_backoff_duration(40, cap, 0.5),
            Duration::from_secs(8)
        );
        assert_eq!(
            exponential_backoff_duration(u32::MAX, cap, f64::NAN),
            Duration::ZERO
        );
        Ok(())
    }
}
