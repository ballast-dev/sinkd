//! `sinkd-srv init` — scaffold system config from the template.

use std::path::PathBuf;

use clap::ArgMatches;
use sinkd_core::{
    init::{
        InitOptions, SYSTEM_TEMPLATE, SYSTEM_TEMPLATE_DISK, render, toml_string_array_body,
    },
    outcome::Outcome,
};

use crate::parameters::ServerParameters;

pub fn run(sub: &ArgMatches, _parameters: &ServerParameters) -> Outcome<()> {
    let users_csv = sub
        .get_one::<String>("users")
        .ok_or_else(|| "server init: --users is required".to_string())?;
    let users: Vec<String> = users_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if users.is_empty() {
        return sinkd_core::bad!("server init: --users must contain at least one name");
    }
    let server_addr = sub
        .get_one::<String>("server-addr")
        .map_or("0.0.0.0", String::as_str);
    let force = sub.get_flag("force");

    let target = sub
        .get_one::<String>("config")
        .map_or_else(default_system_target, PathBuf::from);

    let users_body = toml_string_array_body(&users);
    render(&InitOptions {
        target_path: target,
        template_disk: Some(std::path::Path::new(SYSTEM_TEMPLATE_DISK)),
        template_embedded: SYSTEM_TEMPLATE,
        substitutions: &[
            ("server_addr", server_addr.to_string()),
            ("users", users_body),
        ],
        force,
    })
}

fn default_system_target() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/opt/sinkd/sinkd.conf")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\sinkd\sinkd.conf")
    } else {
        PathBuf::from("/etc/sinkd.conf")
    }
}
