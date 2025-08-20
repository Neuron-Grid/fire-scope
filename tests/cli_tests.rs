use clap::Parser;
use fire_scope::cli::Cli;

#[test]
fn cli_parses_country_and_format() {
    // 有効な最小引数（国コード + フォーマット）
    let args = [
        "fire-scope",
        "-c",
        "jp",
        "-f",
        "nft",
        "--concurrency",
        "3",
    ];

    let cli = Cli::parse_from(&args);
    let cc = cli.country_codes.expect("country required");
    assert_eq!(cc, vec!["JP".to_string()]);
    assert_eq!(cli.output_format, "nft".to_string());
    assert_eq!(cli.concurrency, 3usize);
    assert!(!cli.overlap);
    assert!(cli.as_numbers.is_none());
}

#[test]
fn cli_parses_as_numbers_and_overlap_requirements() {
    // overlap 指定時は両方必須（ここでは両方指定して成功を確認）
    let args = [
        "fire-scope",
        "-o",
        "-c",
        "us",
        "-a",
        "65000",
        "--format",
        "txt",
    ];
    let cli = Cli::parse_from(&args);
    assert!(cli.overlap);
    assert_eq!(cli.country_codes.unwrap(), vec!["US".to_string()]);
    assert_eq!(cli.as_numbers.unwrap(), vec![65000]);
    assert_eq!(cli.output_format, "txt".to_string());
}

