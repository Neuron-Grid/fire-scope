use clap::Parser;

fn parse_country_code(s: &str) -> Result<String, String> {
    let upper = s.to_ascii_uppercase();
    let valid = upper.chars().all(|c| c.is_ascii_alphabetic());
    if !valid {
        return Err("Country code must be alphabetic (A-Z)".into());
    }
    let len = upper.len();
    if !(len == 2 || len == 3) {
        return Err("Country code length must be 2 or 3".into());
    }
    Ok(upper)
}

/// CLIの定義
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "This tool can be used to obtain IP addresses by country or by AS number."
)]
pub struct Cli {
    #[arg(
        short = 'c',
        long = "country",
        required_unless_present_any = ["as_numbers", "overlap"],
        required = false,
        num_args = 1..,
        value_parser = parse_country_code,
        help = "Specify the country codes.\nExample: jp br us"
    )]
    pub country_codes: Option<Vec<String>>,

    #[arg(
        short = 'a',
        long = "as-number",
        required_unless_present_any = ["country_codes", "overlap"],
        required = false,
        value_parser = clap::value_parser!(u32),
        num_args = 1..,
        help = "Specify AS numbers.\nExample: 0000 1234"
    )]
    pub as_numbers: Option<Vec<u32>>,

    #[arg(
        short = 'o',
        long = "overlap",
        help = "Write down the IP addresses of the overlapping country and AS numbers in a file of your choice.\nBoth the -c and -a arguments must be specified.",
        required = false,
        default_value = "false",
        requires("country_codes"),
        requires("as_numbers")
    )]
    pub overlap: bool,

    #[arg(
        short = 'f',
        long = "format",
        default_value = "txt",
        required = false,
        hide_default_value = true,
        value_parser = ["txt", "nft"],
        help = "Select output format: 'txt' or 'nft'.\ndefault: txt"
    )]
    pub output_format: String,

    #[arg(
        long = "max-retries",
        help = "Maximum HTTP retry attempts for downloads.",
        required = false,
        default_value_t = 6u32,
        value_parser = clap::value_parser!(u32)
    )]
    pub max_retries: u32,

    #[arg(
        long = "max-backoff-sec",
        help = "Cap for exponential backoff seconds per retry.",
        required = false,
        default_value_t = 16u64,
        value_parser = clap::value_parser!(u64)
    )]
    pub max_backoff_sec: u64,

    #[arg(
        long = "http-timeout-secs",
        help = "HTTP request total timeout in seconds.",
        required = false,
        default_value_t = 20u64,
        value_parser = clap::value_parser!(u64)
    )]
    pub http_timeout_secs: u64,

    #[arg(
        long = "connect-timeout-secs",
        help = "HTTP connect timeout in seconds.",
        required = false,
        default_value_t = 10u64,
        value_parser = clap::value_parser!(u64)
    )]
    pub connect_timeout_secs: u64,

    #[arg(
        long = "concurrency",
        short = 'C',
        help = "Max concurrent AS queries.",
        required = false,
        default_value_t = 5usize,
        value_parser = clap::value_parser!(usize).range(1..=64)
    )]
    pub concurrency: usize,

    #[arg(
        long = "continue-on-partial",
        help = "Continue with successfully downloaded RIR files even if some downloads fail.",
        required = false,
        default_value_t = false
    )]
    pub continue_on_partial: bool,

    #[arg(
        long = "debug",
        short = 'd',
        help = "Enable verbose debug output to stderr.",
        required = false,
        default_value_t = false
    )]
    pub debug: bool,
}
