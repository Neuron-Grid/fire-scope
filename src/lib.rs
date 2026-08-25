//! Command-line entry point for generating firewall IP lists.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(clippy::all, clippy::pedantic)]

mod asn;
mod cli;
mod commands;
mod country;
mod diagnostics;
mod error;
mod fetch;
mod ip;
mod ipv4_utils;
mod output;
mod output_file;
mod output_render;
mod overlap;
mod parse;
mod rir_download;

pub use commands::run;
