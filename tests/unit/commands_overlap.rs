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
