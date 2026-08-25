use super::{Cli, Command, ListCommand};
use crate::output::OutputFormat;
use clap::Parser;

#[test]
fn parses_nested_country_command() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from([
        "fire-scope",
        "--format",
        "nft",
        "list",
        "country",
        "jp",
        "usa",
    ])?;

    assert_eq!(cli.output_format, OutputFormat::Nft);
    assert!(matches!(
        cli.command,
        Command::List {
            target: ListCommand::Country(args)
        } if args.country_codes == ["JP", "USA"]
    ));
    Ok(())
}

#[test]
fn parses_nested_asn_and_overlap_commands() -> Result<(), clap::Error> {
    let asn = Cli::try_parse_from([
        "fire-scope",
        "list",
        "asn",
        "1234",
        "65000",
        "--concurrency",
        "64",
    ])?;
    assert!(matches!(
        asn.command,
        Command::List {
            target: ListCommand::Asn(args)
        } if args.as_numbers.iter().map(|number| number.get()).eq([1234, 65000])
            && args.query.concurrency == 64
    ));

    let overlap =
        Cli::try_parse_from(["fire-scope", "overlap", "--country", "jp", "--asn", "1234"])?;
    assert_eq!(overlap.output_format, OutputFormat::Txt);
    assert_eq!(overlap.http_timeout_secs.get(), 20);
    assert_eq!(overlap.connect_timeout_secs.get(), 10);
    assert!(!overlap.debug);
    assert!(matches!(
        overlap.command,
        Command::Overlap(args)
            if args.country_codes == ["JP"]
                && args.as_numbers.iter().map(|number| number.get()).eq([1234])
                && args.rir.attempts.get() == 6
                && args.rir.max_backoff_secs.get() == 16
                && !args.rir.continue_on_partial
                && args.query.concurrency == 5
    ));
    Ok(())
}

#[test]
fn accepts_typed_boundary_values() -> Result<(), clap::Error> {
    let country = Cli::try_parse_from([
        "fire-scope",
        "--format",
        "txt",
        "list",
        "country",
        "jp",
        "--rir-attempts",
        "1",
        "--max-backoff-secs",
        "1",
    ])?;
    assert_eq!(country.output_format, OutputFormat::Txt);
    assert!(matches!(
        country.command,
        Command::List {
            target: ListCommand::Country(args)
        } if args.rir.attempts.get() == 1 && args.rir.max_backoff_secs.get() == 1
    ));

    for (concurrency, expected) in [("1", 1), ("64", 64)] {
        let cli = Cli::try_parse_from([
            "fire-scope",
            "list",
            "asn",
            "4294967295",
            "--concurrency",
            concurrency,
        ])?;
        assert!(matches!(
            cli.command,
            Command::List {
                target: ListCommand::Asn(args)
            } if args.as_numbers.iter().map(|number| number.get()).eq([u32::MAX])
                && args.query.concurrency == expected
        ));
    }
    Ok(())
}

#[test]
fn rejects_invalid_boundaries_and_legacy_flags() {
    for args in [
        vec!["fire-scope", "list", "country", "J"],
        vec!["fire-scope", "list", "country", "JPNN"],
        vec!["fire-scope", "list", "country", "J1"],
        vec!["fire-scope", "list", "country", "日本"],
        vec!["fire-scope", "list", "country", "JP", "--rir-attempts", "0"],
        vec![
            "fire-scope",
            "list",
            "country",
            "JP",
            "--max-backoff-secs",
            "0",
        ],
        vec!["fire-scope", "list", "asn", "invalid"],
        vec!["fire-scope", "list", "asn", "0"],
        vec!["fire-scope", "list", "asn", "-1"],
        vec!["fire-scope", "list", "asn", "4294967296"],
        vec!["fire-scope", "overlap", "--country", "JP", "--asn", "0"],
        vec!["fire-scope", "list", "asn", "1", "-C", "0"],
        vec!["fire-scope", "list", "asn", "1", "-C", "65"],
        vec!["fire-scope", "--format", "json", "list", "asn", "1"],
        vec!["fire-scope", "--http-timeout-secs", "0", "list", "asn", "1"],
        vec![
            "fire-scope",
            "--connect-timeout-secs",
            "0",
            "list",
            "asn",
            "1",
        ],
        vec!["fire-scope", "-c", "jp"],
        vec!["fire-scope", "-a", "1234"],
        vec!["fire-scope", "-o"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
}
