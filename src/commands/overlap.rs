use super::download_rir_texts;
use crate::asn::fetch_as_results;
use crate::cli::OverlapArgs;
use crate::country::{parse_country_map, select_country_ips};
use crate::diagnostics::DebugOutput;
use crate::error::AppError;
use crate::ip::IpSets;
use crate::output::{OutputFormat, write_overlap_to_file};
use crate::overlap::find_overlaps;
use futures::{Stream, StreamExt, TryStreamExt};
use reqwest::Client;

pub(super) async fn run(
    client: &Client,
    args: OverlapArgs,
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
    let country_ips = selection.merged_ips();

    let as_ips = collect_as_ips(fetch_as_results(
        client,
        &args.as_numbers,
        args.query.concurrency,
        debug,
    ))
    .await?;

    let overlaps = find_overlaps(&country_ips, &as_ips)?;
    write_overlap_to_file(
        &args.country_codes.join("_"),
        &args.as_numbers,
        &overlaps,
        output_format,
        debug,
    )
    .await
}

async fn collect_as_ips(
    results: impl Stream<Item = (u32, Result<IpSets, AppError>)>,
) -> Result<IpSets, AppError> {
    results
        .map(|(as_number, result)| {
            result.map_err(|error| AppError::Other(format!("AS{as_number}: {error}")))
        })
        .try_fold(
            IpSets::default(),
            |sets, ips| async move { Ok(sets.merge(ips)) },
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::collect_as_ips;
    use crate::error::AppError;
    use crate::ip::IpSets;
    use futures::stream;
    use ipnet::IpNet;
    use std::error::Error;
    use std::str::FromStr;

    #[tokio::test]
    async fn as_collection_merges_successes_and_propagates_failure() -> Result<(), Box<dyn Error>> {
        let first = [IpNet::from_str("192.0.2.0/24")?]
            .into_iter()
            .collect::<IpSets>();
        let second = [IpNet::from_str("2001:db8::/32")?]
            .into_iter()
            .collect::<IpSets>();
        let merged = collect_as_ips(stream::iter([
            (1, Ok::<IpSets, AppError>(first)),
            (2, Ok::<IpSets, AppError>(second)),
        ]))
        .await?;
        assert_eq!(merged.ipv4().len(), 1);
        assert_eq!(merged.ipv6().len(), 1);

        let failure = collect_as_ips(stream::iter([(
            64512,
            Err(AppError::Other("failed".into())),
        )]))
        .await
        .err()
        .map(|error| error.to_string());
        assert!(failure.is_some_and(|message| message.contains("AS64512: failed")));
        Ok(())
    }
}
