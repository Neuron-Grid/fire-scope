use std::{io, net::AddrParseError, num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;
use tokio::sync::AcquireError;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum AppError {
    // IOまわりのエラー
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    // ネットワーク関係のエラー (reqwest 等)
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    // UTF-8パースなどの文字列変換エラー
    #[error("String conversion error: {0}")]
    Utf8(#[from] FromUtf8Error),

    // 汎用的なパースエラー
    #[error("Parse error: {0}")]
    ParseError(String),

    // 特定の入力が不正だった場合など
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    // その他、文字列メッセージだけを格納した汎用エラー
    #[error("{0}")]
    Other(String),

    // acquire_owned().await? のエラー
    #[error("Semaphore acquire error: {0}")]
    SemaphoreError(#[from] AcquireError),

    // tokio::spawn(…).await? のエラー
    #[error("Task join error: {0}")]
    JoinError(#[from] JoinError),

    // IPv4Addr, Ipv6Addr などのパース失敗
    #[error("Address parse error: {0}")]
    AddrParseError(#[from] AddrParseError),

    // 文字列 → 数値パース失敗
    #[error("Integer parse error: {0}")]
    ParseIntError(#[from] ParseIntError),
}
