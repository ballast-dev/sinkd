//! sinkd library — deployable sync daemon (Zenoh + rsync).
//!
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
//!
//! **Binary**: `sinkd` — CLI with `sinkd server` / `sinkd client` (and `add`, `rm`, …).

#[macro_use]
pub mod fancy;
#[macro_use]
pub mod outcome;
pub mod cli;
pub mod client;
pub mod config;
pub mod ipc;
pub mod ops;
pub mod parameters;
pub mod rsync;
pub mod server;
pub mod shiplog;
pub mod test_hooks;
pub mod time;

pub use outcome::Outcome;
