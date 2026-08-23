use super::download_rir_texts;
use crate::cli::CountryArgs;
use crate::country::{parse_country_map, select_country_ips};
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::output::{OutputFormat, write_ip_lists_to_files};
use futures::future::try_join_all;
use reqwest::Client;

pub(super) async fn run(
    client: &Client,
    args: CountryArgs,
    output_format: OutputFormat,
    debug: DebugOutput,
) -> Result<(), AppError> {
    let rir_texts = download_rir_texts(client, args.rir, debug).await?;
    let country_map = parse_country_map(&rir_texts, &args.country_codes).await?;
    let selection = select_country_ips(&country_map, &args.country_codes);
    selection
        .missing_codes
        .iter()
        .for_each(|code| debug.log(format!("No IPs found for country code: {code}")));

    let writes = selection.found.into_iter().map(|(country_code, ip_sets)| {
        write_ip_lists_to_files(country_code, ip_sets, output_format, debug)
    });

    try_join_all(writes).await.map(|_| ())
}
