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
