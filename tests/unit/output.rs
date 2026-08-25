use super::{OutputFormat, format_as_numbers};
use std::num::NonZeroU32;

#[test]
fn output_metadata_is_deterministic() -> Result<(), std::num::TryFromIntError> {
    assert_eq!(OutputFormat::Txt.extension(), "txt");
    assert_eq!(OutputFormat::Nft.extension(), "nft");
    assert_eq!(
        format_as_numbers(&[NonZeroU32::try_from(1234)?, NonZeroU32::try_from(5678)?,]),
        "1234_5678"
    );
    Ok(())
}
