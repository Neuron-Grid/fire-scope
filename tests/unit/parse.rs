use super::{parse_all_country_codes, parse_ipv4_range_to_cidrs};
use crate::error::AppError;

#[test]
fn parses_selected_allocations_by_family() -> Result<(), AppError> {
    let text = [
        "# comment",
        "apnic|JP|ipv4|1.2.3.0|256|20200101|allocated",
        "apnic|JP|ipv4|1.2.4.0|256|20200101|available",
        "apnic|jp|IPV6|2001:db8::|32|20200101|ASSIGNED",
        "ripe|US|ipv4|203.0.113.0|256|20200101|allocated",
        "apnic|JP|asn|12345|1|20200101|allocated",
    ]
    .join("\n");

    let countries = parse_all_country_codes(&[text], &["JP".to_owned()])?;
    let sets = countries
        .get("JP")
        .ok_or_else(|| AppError::Other("JP test records are missing".into()))?;
    assert_eq!(
        sets.ipv4()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["1.2.3.0/24"]
    );
    assert_eq!(
        sets.ipv6()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["2001:db8::/32"]
    );
    Ok(())
}

#[test]
fn ignores_non_allocations_and_only_parses_selected_records() -> Result<(), AppError> {
    let ignored = [
        "# comment",
        "short|line",
        "apnic|JP|ipv4|10.0.0.0|256|20200101|reserved",
        "apnic|JP|ipv4|10.0.1.0|256|20200101|available",
        "apnic|JP|asn|1234|1|20200101|allocated",
    ]
    .join("\n");

    assert!(parse_all_country_codes(&[ignored], &["JP".to_owned()])?.is_empty());
    assert!(
        parse_all_country_codes(
            &["apnic|JP|ipv4|invalid|256|20200101|allocated\n".to_owned()],
            &["JP".to_owned()],
        )
        .is_err()
    );
    assert!(
        parse_all_country_codes(
            &["apnic|US|ipv4|invalid|256|20200101|allocated\n".to_owned()],
            &["JP".to_owned()],
        )
        .is_ok()
    );
    Ok(())
}

#[test]
fn aggregates_selected_country_across_rir_files() -> Result<(), AppError> {
    let texts = [
        "apnic|JP|ipv4|10.0.0.0|128|20200101|allocated\n".to_owned(),
        "apnic|jp|ipv4|10.0.0.128|128|20200101|allocated\n".to_owned(),
    ];

    let countries = parse_all_country_codes(&texts, &["JP".to_owned()])?;
    assert_eq!(
        countries.get("JP").map(|sets| {
            sets.ipv4()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        }),
        Some(vec!["10.0.0.0/24".to_owned()])
    );
    Ok(())
}

#[test]
fn returns_first_rir_error_in_input_order() {
    let invalid_address = "apnic|JP|ipv4|invalid|256|20200101|allocated\n".to_owned();
    let invalid_width = "apnic|JP|ipv4|192.0.2.0|invalid|20200101|allocated\n".to_owned();
    let countries = ["JP".to_owned()];

    let first = parse_all_country_codes(
        &[invalid_address.clone(), invalid_width.clone()],
        &countries,
    )
    .err();
    let second = parse_all_country_codes(&[invalid_width, invalid_address], &countries).err();

    assert!(matches!(first, Some(AppError::AddrParseError(_))));
    assert!(matches!(second, Some(AppError::ParseIntError(_))));
}

#[test]
fn parses_rir_range_with_checked_boundary() -> Result<(), AppError> {
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
