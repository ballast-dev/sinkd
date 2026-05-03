//! sinkd-core — shared sync daemon library (Zenoh + rsync).
//!
//! Binaries: `sinkd` (client) and `sinkd-srv` (server).
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

#[macro_use]
pub mod fancy;
#[macro_use]
pub mod outcome;
pub mod config;
pub mod conflict;
pub mod init;
pub mod ipc;
pub mod parameters;
pub mod rsync;
pub mod shiplog;
pub mod test_hooks;
pub mod time;

pub use outcome::Outcome;
