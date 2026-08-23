#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    fire_scope::run().await
}
