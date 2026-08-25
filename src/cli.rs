use crate::output::OutputFormat;
use clap::{Args, Parser, Subcommand};
use std::num::{NonZeroU32, NonZeroU64};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    propagate_version = true,
    about = "Generate country and AS IP address lists for firewall rules."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,

    #[arg(
        short = 'f',
        long = "format",
        global = true,
        value_enum,
        default_value_t = OutputFormat::Txt,
        help = "Select output format."
    )]
    pub(crate) output_format: OutputFormat,

    #[arg(
        long = "http-timeout-secs",
        global = true,
        default_value = "20",
        help = "HTTP request timeout in seconds."
    )]
    pub(crate) http_timeout_secs: NonZeroU64,

    #[arg(
        long = "connect-timeout-secs",
        global = true,
        default_value = "10",
        help = "HTTP connection timeout in seconds."
    )]
    pub(crate) connect_timeout_secs: NonZeroU64,

    #[arg(
        short = 'd',
        long = "debug",
        global = true,
        help = "Enable debug output on stderr."
    )]
    pub(crate) debug: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Generate IP lists.
    List {
        #[command(subcommand)]
        target: ListCommand,
    },
    /// Generate the overlap between countries and AS numbers.
    Overlap(OverlapArgs),
}

#[derive(Subcommand, Debug)]
pub(crate) enum ListCommand {
    /// Generate country IP lists from RIR allocation data.
    Country(CountryArgs),
    /// Generate AS announced-prefix lists.
    Asn(AsnArgs),
}

#[derive(Args, Debug)]
pub(crate) struct CountryArgs {
    #[arg(required = true, value_parser = parse_country_code)]
    pub(crate) country_codes: Vec<String>,

    #[command(flatten)]
    pub(crate) rir: RirOptions,
}

#[derive(Args, Debug)]
pub(crate) struct AsnArgs {
    #[arg(required = true)]
    pub(crate) as_numbers: Vec<NonZeroU32>,

    #[command(flatten)]
    pub(crate) query: AsnQueryOptions,
}

#[derive(Args, Debug)]
pub(crate) struct OverlapArgs {
    #[arg(
        long = "country",
        required = true,
        num_args = 1..,
        value_parser = parse_country_code
    )]
    pub(crate) country_codes: Vec<String>,

    #[arg(long = "asn", required = true, num_args = 1..)]
    pub(crate) as_numbers: Vec<NonZeroU32>,

    #[command(flatten)]
    pub(crate) rir: RirOptions,

    #[command(flatten)]
    pub(crate) query: AsnQueryOptions,
}

#[derive(Args, Clone, Copy, Debug)]
pub(crate) struct RirOptions {
    #[arg(
        long = "rir-attempts",
        default_value = "6",
        help = "Total download attempts for each RIR file."
    )]
    pub(crate) attempts: NonZeroU32,

    #[arg(
        long = "max-backoff-secs",
        default_value = "16",
        help = "Maximum exponential-backoff delay in seconds."
    )]
    pub(crate) max_backoff_secs: NonZeroU64,

    #[arg(
        long = "continue-on-partial",
        help = "Use successful RIR downloads when some downloads fail."
    )]
    pub(crate) continue_on_partial: bool,
}

#[derive(Args, Clone, Copy, Debug)]
pub(crate) struct AsnQueryOptions {
    #[arg(
        short = 'C',
        long = "concurrency",
        default_value = "5",
        value_parser = parse_concurrency,
        help = "Maximum concurrent AS queries."
    )]
    pub(crate) concurrency: usize,
}

fn parse_concurrency(input: &str) -> Result<usize, String> {
    input
        .parse::<usize>()
        .map_err(|error| error.to_string())
        .and_then(|value| {
            (1..=64)
                .contains(&value)
                .then_some(value)
                .ok_or_else(|| "concurrency must be between 1 and 64".to_string())
        })
}

fn parse_country_code(input: &str) -> Result<String, String> {
    let country_code = input.to_ascii_uppercase();
    let valid_length = matches!(country_code.len(), 2 | 3);
    let valid_characters = country_code.chars().all(|ch| ch.is_ascii_alphabetic());

    (valid_length && valid_characters)
        .then_some(country_code)
        .ok_or_else(|| "country code must contain 2 or 3 ASCII letters".to_string())
}

#[cfg(test)]
#[path = "../tests/unit/cli.rs"]
mod tests;
