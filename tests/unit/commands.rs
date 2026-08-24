use super::apply_rir_failure_policy;

#[test]
fn rir_partial_failure_requires_explicit_continuation() {
    let successful = vec!["data".to_owned()];

    assert!(apply_rir_failure_policy(successful.clone(), 1, false).is_err());
    assert_eq!(
        apply_rir_failure_policy(successful.clone(), 1, true).ok(),
        Some(successful)
    );
    assert!(apply_rir_failure_policy(Vec::new(), 5, true).is_err());
}
