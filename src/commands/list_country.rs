use super::rir::load_country_map;
use crate::cli::CountryArgs;
use crate::country::select_country_ips;
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::output::{OutputFormat, write_ip_lists_to_files};
use reqwest::Client;

pub(super) async fn run(
    client: &Client,
    args: CountryArgs,
    output_format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let country_map = load_country_map(client, &args.country_codes, args.rir, debug).await?;
    let selection = select_country_ips(&country_map, &args.country_codes);
    selection
        .missing_codes
        .iter()
        .for_each(|code| debug.log(format!("No IPs found for country code: {code}")));

    for (country_code, ip_sets) in selection.found {
        write_ip_lists_to_files(country_code, ip_sets, output_format, debug).await?;
    }
    Ok(())
}
