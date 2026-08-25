use super::finalize_as_processing;
use crate::error::AppError;
use std::num::NonZeroU32;

#[test]
fn finalization_reports_partial_failure() -> Result<(), std::num::TryFromIntError> {
    let outcomes = vec![
        (NonZeroU32::try_from(1234)?, Ok::<(), AppError>(())),
        (
            NonZeroU32::try_from(5678)?,
            Err(AppError::Other("boom".into())),
        ),
    ];
    let message = finalize_as_processing(outcomes)
        .err()
        .map(|error| error.to_string());

    assert!(message.as_deref().is_some_and(|text| {
        text.contains("Failed to process 1 AS number(s)") && text.contains("AS5678: boom")
    }));
    Ok(())
}
