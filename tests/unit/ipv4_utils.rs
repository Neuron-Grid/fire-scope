use super::{ipv4_summarize_range, largest_ipv4_block, parse_ipv4_range_to_cidrs};
use std::error::Error;

#[test]
fn chooses_largest_aligned_block() -> Result<(), Box<dyn Error>> {
    assert_eq!(largest_ipv4_block(0, 255)?, 24);
    assert_eq!(largest_ipv4_block(0, 511)?, 23);
    assert_eq!(largest_ipv4_block(1, 1)?, 32);
    assert!(largest_ipv4_block(2, 1).is_err());
    Ok(())
}

#[test]
fn summarizes_valid_ranges_and_rejects_reversed_ranges() -> Result<(), Box<dyn Error>> {
    let entire_family = ipv4_summarize_range(0, u32::MAX)?;
    let full_block = ipv4_summarize_range(0, 255)?;
    let unaligned = ipv4_summarize_range(1, 3)?;

    assert_eq!(
        entire_family
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["0.0.0.0/0"]
    );
    assert_eq!(
        full_block
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["0.0.0.0/24"]
    );
    assert_eq!(
        unaligned
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["0.0.0.1/32", "0.0.0.2/31"]
    );
    assert!(ipv4_summarize_range(2, 1).is_err());
    Ok(())
}

#[test]
fn parses_rir_range_with_checked_boundary() -> Result<(), Box<dyn Error>> {
    let single = parse_ipv4_range_to_cidrs("1.2.3.4", "1")?;
    let block = parse_ipv4_range_to_cidrs("1.2.3.0", "256")?;

    assert_eq!(
        single.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["1.2.3.4/32"]
    );
    assert_eq!(
        block.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["1.2.3.0/24"]
    );
    assert!(parse_ipv4_range_to_cidrs("1.2.3.4", "0").is_err());
    assert!(parse_ipv4_range_to_cidrs("255.255.255.255", "2").is_err());
    Ok(())
}
