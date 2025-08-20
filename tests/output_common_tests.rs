use fire_scope::output_common::{make_header, sanitize_identifier, write_list_nft, write_list_txt};
use ipnet::IpNet;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::fs;

fn ipnet(s: &str) -> IpNet {
    match IpNet::from_str(s) {
        Ok(n) => n,
        Err(e) => panic!("failed to parse {s}: {e}"),
    }
}

#[test]
fn sanitize_identifier_examples() {
    assert_eq!(sanitize_identifier(" asn-123 "), "asn_123");
    assert_eq!(sanitize_identifier("日本語混在!"), "UNKNOWN");
    assert_eq!(sanitize_identifier("a".repeat(100).as_str()).len(), 64);
}

#[tokio::test]
async fn write_txt_and_nft_outputs_expected_content() {
    // 出力ディレクトリ
    let dir = PathBuf::from("target/test-output");
    if let Err(e) = fs::create_dir_all(&dir).await {
        panic!("mkdir failed: {e}")
    }

    // IPv4/IPv6の小さなセット
    let mut set = BTreeSet::new();
    set.insert(ipnet("192.0.2.0/24"));
    set.insert(ipnet("2001:db8::/32"));
    let header = make_header("2025-01-01 00:00:00", "JP", "AS1234");

    // TXT
    let txt_path = dir.join(format!("list_{}.txt", rand::random::<u64>()));
    if let Err(e) = write_list_txt(&txt_path, &set, &header).await {
        panic!("write txt failed: {e}")
    }
    let txt_content = fs::read_to_string(&txt_path)
        .await
        .unwrap_or_else(|e| panic!("read txt: {e}"));
    assert!(txt_content.contains("# Country Code: JP"));
    assert!(txt_content.contains("192.0.2.0/24"));
    assert!(txt_content.contains("2001:db8::/32"));

    // NFT（define名はファイル名stemをsanitize）
    let nft_name = "Te$st-List 01";
    let nft_path = dir.join(format!("{}-{}.nft", nft_name, rand::random::<u64>()));
    if let Err(e) = write_list_nft(&nft_path, &set, &header).await {
        panic!("write nft failed: {e}")
    }
    let nft_content = fs::read_to_string(&nft_path)
        .await
        .unwrap_or_else(|e| panic!("read nft: {e}"));
    assert!(nft_content.contains("# AS Number: AS1234"));
    assert!(nft_content.contains("192.0.2.0/24"));
    assert!(nft_content.contains("2001:db8::/32"));
    // ファイル名に乱数サフィックスが付くため、prefixのみ確認
    assert!(nft_content.contains("define Te_st_List_01"));
}
