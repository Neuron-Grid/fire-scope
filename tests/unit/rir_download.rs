use super::partition_downloads;
use crate::error::AppError;

#[test]
fn partitions_download_results_without_erasing_errors() {
    let (texts, failures) = partition_downloads(
        &["https://success.example", "https://failure.example"],
        vec![Ok("body".into()), Err(AppError::Other("failed".into()))],
    );

    assert_eq!(texts, ["body"]);
    assert!(matches!(
        failures.as_slice(),
        [(url, AppError::Other(error))]
            if url == "https://failure.example" && error == "failed"
    ));
}
