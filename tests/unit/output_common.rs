use super::{
    atomic_write, render_nft, render_txt, sanitize_identifier, write_list_nft, write_list_txt,
};
use crate::error::AppError;
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::fs;

fn sample_ipnets() -> BTreeSet<IpNet> {
    ["192.0.2.0/24", "2001:db8::/32"]
        .into_iter()
        .filter_map(|cidr| IpNet::from_str(cidr).ok())
        .collect()
}

#[test]
fn renderers_and_identifier_are_deterministic() {
    let ipnets = sample_ipnets();
    let empty = BTreeSet::new();

    assert_eq!(
        render_txt(&ipnets, "# header\n"),
        "# header\n192.0.2.0/24\n2001:db8::/32\n"
    );
    assert_eq!(
        render_nft(&ipnets, "# header\n", "routes"),
        "# header\ndefine routes = {\n    192.0.2.0/24,\n    2001:db8::/32\n}\n"
    );
    assert_eq!(render_txt(&empty, "# header\n"), "# header\n\n");
    assert_eq!(
        render_nft(&empty, "# header\n", "routes"),
        "# header\ndefine routes = {\n}\n"
    );
    assert_eq!(sanitize_identifier("---"), "UNKNOWN");
    assert_eq!(sanitize_identifier(&"a".repeat(64)), "a".repeat(64));
    assert_eq!(sanitize_identifier(&"a".repeat(65)), "a".repeat(64));
}

#[tokio::test]
async fn txt_and_nft_writers_replace_content_without_temporary_files() -> Result<(), AppError> {
    let directory = PathBuf::from("target/test-output")
        .join(format!("output-common-{}", rand::random::<u64>()));
    fs::create_dir_all(&directory).await?;
    let txt_path = directory.join("routes.txt");
    let nft_path = directory.join("routes.nft");
    let ipnets = sample_ipnets();

    atomic_write(&txt_path, b"obsolete").await?;
    atomic_write(&nft_path, b"obsolete").await?;
    write_list_txt(&txt_path, &ipnets, "# header\n").await?;
    write_list_nft(&nft_path, &ipnets, "# header\n").await?;
    assert_eq!(
        fs::read_to_string(&txt_path).await?,
        render_txt(&ipnets, "# header\n")
    );
    assert_eq!(
        fs::read_to_string(&nft_path).await?,
        render_nft(&ipnets, "# header\n", "routes")
    );

    let mut entries = fs::read_dir(&directory).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(!name.contains(".tmp."));
    }

    fs::remove_dir_all(&directory).await?;
    Ok(())
}

#[tokio::test]
async fn rename_failure_preserves_destination_and_removes_temporary_file() -> Result<(), AppError> {
    let directory = PathBuf::from("target/test-output")
        .join(format!("output-failure-{}", rand::random::<u64>()));
    let destination = directory.join("routes.txt");
    fs::create_dir_all(&destination).await?;
    let existing = destination.join("existing");
    fs::write(&existing, b"old").await?;

    assert!(atomic_write(&destination, b"new").await.is_err());
    assert_eq!(fs::read(&existing).await?, b"old");

    let mut entries = fs::read_dir(&directory).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(!name.contains(".tmp."));
    }

    fs::remove_dir_all(&directory).await?;
    Ok(())
}
