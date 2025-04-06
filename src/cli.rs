use clap::Parser;

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
        short = 'm',
        long = "mode",
        default_value = "overwrite",
        required = false,
        hide_default_value = true,
        help = "Select file output mode: 'append' or 'overwrite'.\ndefault: overwrite"
    )]
    pub mode: String,

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
        help = "Select output format: 'txt' or 'nft'.\ndefault: txt"
    )]
    pub output_format: String,
}
