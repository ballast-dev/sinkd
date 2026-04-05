//! Command-line definition and dispatch.

mod build;
mod client;
mod common;
mod dispatch;
mod server;

pub use build::build_sinkd;
pub use dispatch::{dispatch_sinkd_matches, run_sinkd};
