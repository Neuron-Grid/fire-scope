use super::{OutputFormat, format_as_numbers};

#[test]
fn output_metadata_is_deterministic() {
    assert_eq!(OutputFormat::Txt.extension(), "txt");
    assert_eq!(OutputFormat::Nft.extension(), "nft");
    assert_eq!(format_as_numbers(&[1234, 5678]), "1234_5678");
}
