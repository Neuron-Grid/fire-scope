use super::{DownloadOutcome, classify_download};
use crate::error::AppError;

#[test]
fn classifies_download_results_without_task_wrapping() {
    assert!(matches!(
        classify_download("https://example.test", Ok("body".into())),
        DownloadOutcome::Success(text) if text == "body"
    ));
    assert!(matches!(
        classify_download(
            "https://example.test",
            Err(AppError::Other("failed".into()))
        ),
        DownloadOutcome::Failure { url, error }
            if url == "https://example.test" && error.contains("failed")
    ));
}
