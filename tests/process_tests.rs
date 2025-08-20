use fire_scope::process::{parse_and_collect_ips, process_country_code_from_map};
use fire_scope::common::OutputFormat;
use ipnet::IpNet;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::fs;

fn ipnet(s: &str) -> IpNet {
    IpNet::from_str(s).unwrap_or_else(|e| panic!("failed to parse {s}: {e}"))
}

#[test]
fn parse_and_collect_ips_aggregates_and_filters() {
    let rir1 = "apnic|JP|ipv4|10.0.0.0|128|20200101|allocated\n"; // /25
    let rir2 = "apnic|JP|ipv4|10.0.0.128|128|20200101|allocated\n"; // /25
    let rir3 = "apnic|JP|ipv6|2001:db8::|32|20200101|assigned\n"; // v6
    let rir4 = "apnic|JP|ipv4|10.0.1.0|256|20200101|available\n"; // skip
    let texts = vec![rir1.to_string(), rir2.to_string(), rir3.to_string(), rir4.to_string()];

    let (v4, v6) = parse_and_collect_ips("JP", &texts).unwrap();
    let v4s: Vec<String> = v4.iter().map(|n| n.to_string()).collect();
    let v6s: Vec<String> = v6.iter().map(|n| n.to_string()).collect();

    assert!(v4s.contains(&"10.0.0.0/24".to_string()));
    assert!(!v4s.contains(&"10.0.0.0/25".to_string()));
    assert!(!v4s.contains(&"10.0.0.128/25".to_string()));
    assert!(v6s.contains(&"2001:db8::/32".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn process_country_code_from_map_writes_files() {
    // 一意な国コード名（ファイル名重複回避）
    let cc = format!("ZZTEST{}", rand::random::<u32>());
    let mut map: HashMap<String, (Vec<IpNet>, Vec<IpNet>)> = HashMap::new();
    map.insert(
        cc.clone(),
        (
            vec![ipnet("203.0.113.0/25"), ipnet("203.0.113.128/25")], // aggregate→/24
            vec![ipnet("2001:db8::/32")],
        ),
    );

    // 実行（TXT出力）
    process_country_code_from_map(&cc, &map, OutputFormat::Txt)
        .await
        .unwrap_or_else(|e| panic!("process failed: {e}"));

    let v4_path = format!("IPv4_{}.txt", cc);
    let v6_path = format!("IPv6_{}.txt", cc);
    let v4 = fs::read_to_string(&v4_path).await.unwrap_or_else(|e| panic!("read v4: {e}"));
    let v6 = fs::read_to_string(&v6_path).await.unwrap_or_else(|e| panic!("read v6: {e}"));

    assert!(v4.contains("203.0.113.0/24"));
    assert!(v6.contains("2001:db8::/32"));

    // 片付け
    let _ = fs::remove_file(&v4_path).await;
    let _ = fs::remove_file(&v6_path).await;
}
