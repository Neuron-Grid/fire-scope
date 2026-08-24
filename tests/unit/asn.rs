use super::{
    buffered_results, extract_prefixes_from_arin, extract_prefixes_from_ripe_stat,
    finalize_as_processing,
};
use crate::error::AppError;
use futures::StreamExt;
use serde_json::json;

#[test]
fn pure_extractors_ignore_invalid_entries() {
    let ripe = json!({
        "data": {"prefixes": [
            {"prefix": "192.0.2.0/24"},
            {"prefix": "invalid"},
            {"other": "missing"}
        ]}
    });
    let arin = json!({
        "arin_originas0_networkSearchResults": [
            {"ipVersion": "v4", "v4prefix": "198.51.100.0", "length": 24},
            {"ipVersion": "v6", "v6prefix": "2001:db8::", "length": 32},
            {"ipVersion": "v4", "v4prefix": "invalid", "length": 24}
        ]
    });

    assert_eq!(extract_prefixes_from_ripe_stat(&ripe).len(), 1);
    assert_eq!(extract_prefixes_from_arin(&arin).len(), 2);
}

#[tokio::test]
async fn buffered_fetch_preserves_input_order_and_failure() {
    let as_numbers = [3, 1, 2];
    let results = buffered_results(&as_numbers, 2, |as_number| async move {
        if as_number == 1 {
            Err(AppError::Other("failed".into()))
        } else {
            Ok(as_number)
        }
    })
    .collect::<Vec<_>>()
    .await;

    assert_eq!(
        results
            .iter()
            .map(|(as_number, _)| *as_number)
            .collect::<Vec<_>>(),
        as_numbers
    );
    assert!(results.get(1).is_some_and(|(_, result)| result.is_err()));
}

#[test]
fn finalization_reports_partial_failure() {
    let outcomes = vec![
        (1234, Ok::<(), AppError>(())),
        (5678, Err(AppError::Other("boom".into()))),
    ];
    let message = finalize_as_processing(outcomes)
        .err()
        .map(|error| error.to_string());

    assert!(message.as_deref().is_some_and(|text| {
        text.contains("Failed to process 1 AS number(s)") && text.contains("AS5678: boom")
    }));
}
