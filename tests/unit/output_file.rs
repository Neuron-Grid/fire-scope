use super::{atomic_write, atomic_write_with_limit};
use crate::error::AppError;
use crate::output_render::{nft_chunks, txt_chunks};
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;

fn sample_ipnets() -> Result<BTreeSet<IpNet>, Box<dyn Error>> {
    Ok([
        "192.0.2.0/24".parse::<IpNet>()?,
        "2001:db8::/32".parse::<IpNet>()?,
    ]
    .into_iter()
    .collect())
}

async fn assert_no_temporary_files(directory: &Path) -> Result<(), AppError> {
    let mut entries = fs::read_dir(directory).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(!name.contains(".tmp."));
    }
    Ok(())
}

#[tokio::test]
async fn txt_and_nft_chunks_replace_content_without_temporary_files() -> Result<(), Box<dyn Error>>
{
    let directory =
        PathBuf::from("target/test-output").join(format!("output-file-{}", rand::random::<u64>()));
    fs::create_dir_all(&directory).await?;
    let txt_path = directory.join("routes.txt");
    let nft_path = directory.join("routes.nft");
    let ipnets = sample_ipnets()?;

    atomic_write(&txt_path, ["obsolete".to_owned()]).await?;
    atomic_write(&nft_path, ["obsolete".to_owned()]).await?;
    atomic_write(&txt_path, txt_chunks(&ipnets, "# header\n")).await?;
    atomic_write(&nft_path, nft_chunks(&ipnets, "# header\n", "routes")).await?;

    assert_eq!(
        fs::read_to_string(&txt_path).await?,
        "# header\n192.0.2.0/24\n2001:db8::/32\n"
    );
    assert_eq!(
        fs::read_to_string(&nft_path).await?,
        "# header\ndefine routes = {\n    192.0.2.0/24,\n    2001:db8::/32\n}\n"
    );
    assert_no_temporary_files(&directory).await?;

    fs::remove_dir_all(&directory).await?;
    Ok(())
}

#[tokio::test]
async fn rename_failure_preserves_destination_and_removes_temporary_file() -> Result<(), AppError> {
    let directory = PathBuf::from("target/test-output")
        .join(format!("output-rename-failure-{}", rand::random::<u64>()));
    let destination = directory.join("routes.txt");
    fs::create_dir_all(&destination).await?;
    let existing = destination.join("existing");
    fs::write(&existing, b"old").await?;

    assert!(
        atomic_write(&destination, ["new".to_owned()])
            .await
            .is_err()
    );
    assert_eq!(fs::read(&existing).await?, b"old");
    assert_no_temporary_files(&directory).await?;

    fs::remove_dir_all(&directory).await?;
    Ok(())
}

#[tokio::test]
async fn limit_failure_preserves_destination_and_removes_temporary_file() -> Result<(), AppError> {
    let directory = PathBuf::from("target/test-output")
        .join(format!("output-limit-failure-{}", rand::random::<u64>()));
    fs::create_dir_all(&directory).await?;
    let destination = directory.join("routes.txt");
    fs::write(&destination, b"old").await?;

    let error = atomic_write_with_limit(&destination, ["1234".to_owned(), "5678".to_owned()], 7)
        .await
        .err()
        .map(|error| error.to_string());

    assert!(error.is_some_and(|message| message.contains("8 bytes > 7 bytes")));
    assert_eq!(fs::read(&destination).await?, b"old");
    assert_no_temporary_files(&directory).await?;

    fs::remove_dir_all(&directory).await?;
    Ok(())
}
