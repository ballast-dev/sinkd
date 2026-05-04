//! `sinkd init` — scaffold system + user config from templates.

use std::path::{Path, PathBuf};

use clap::ArgMatches;
use sinkd_core::{
    config,
    init::{
        InitOptions, SYSTEM_TEMPLATE, SYSTEM_TEMPLATE_DISK, USER_TEMPLATE, USER_TEMPLATE_DISK,
        render, toml_string_array_body,
    },
    outcome::Outcome,
};

use crate::{client, parameters::ClientParameters};

pub fn run(sub: &ArgMatches, parameters: &ClientParameters) -> Outcome<()> {
    let server_addr = sub
        .get_one::<String>("server-addr")
        .map(String::as_str)
        .ok_or("--server-addr is required")?;
    let user = match sub.get_one::<String>("user") {
        Some(s) => s.clone(),
        None => config::get_username()?,
    };
    let watch_arg = sub
        .get_one::<String>("watch")
        .ok_or("--watch is required")?;
    let interval: u64 = sub
        .get_one::<String>("interval")
        .map_or("1", String::as_str)
        .parse()
        .map_err(|e| format!("--interval must be an integer: {e}"))?;
    let force = sub.get_flag("force");

    let watch = PathBuf::from(watch_arg);
    let sys_target = parameters.system_config.as_ref().as_path().to_path_buf();
    let user_target = client::default_user_config_target();
    let users = vec![user];
    let users_body = toml_string_array_body(&users);

    render(&InitOptions {
        target_path: sys_target,
        template_disk: Some(Path::new(SYSTEM_TEMPLATE_DISK)),
        template_embedded: SYSTEM_TEMPLATE,
        substitutions: &[
            ("server_addr", server_addr.to_string()),
            ("users", users_body),
        ],
        force,
    })?;

    render(&InitOptions {
        target_path: user_target,
        template_disk: Some(Path::new(USER_TEMPLATE_DISK)),
        template_embedded: USER_TEMPLATE,
        substitutions: &[
            ("watch", watch.display().to_string()),
            ("interval", interval.to_string()),
        ],
        force,
    })
}
