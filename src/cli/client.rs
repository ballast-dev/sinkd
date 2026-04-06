use clap::ArgMatches;
use std::{path::Path, process::ExitCode};

use crate::client;
use crate::parameters::ClientParameters;

use super::common::{egress, print_subcommand_help};

fn check_path_exists(p: &str) -> bool {
    let p = Path::new(p);
    if p.exists() {
        return true;
    }
    crate::fancy::println(
        &format!("path doesn't exist: {}", p.display()),
        crate::fancy::Attrs::Bold,
        crate::fancy::Colors::Red,
    );
    false
}

fn collect_share_user_paths(submatches: &ArgMatches) -> (Vec<&String>, Vec<&String>) {
    let share_paths = submatches
        .get_many::<String>("share")
        .map(|shares| shares.filter(|p| check_path_exists(p)).collect())
        .unwrap_or_default();
    let user_paths = submatches
        .get_many::<String>("path")
        .map(|paths| paths.filter(|p| check_path_exists(p)).collect())
        .unwrap_or_default();
    (share_paths, user_paths)
}

#[must_use]
pub(super) fn dispatch(sub: &ArgMatches, params: &ClientParameters) -> ExitCode {
    match sub.subcommand() {
        Some(("start", _)) => egress(client::start(params)),
        Some(("restart", _)) => egress(client::restart(params)),
        Some(("stop", _)) => egress(client::stop(params)),
        Some(("add", s)) => {
            let (sp, up) = collect_share_user_paths(s);
            egress(client::add(params, &sp, &up))
        }
        Some(("rm", s)) => {
            let (sp, up) = collect_share_user_paths(s);
            egress(client::rm(params, &sp, &up))
        }
        Some(("adduser", s)) => egress(client::adduser(params, s.get_many::<String>("user"))),
        Some(("rmuser", s)) => egress(client::rmuser(params, s.get_many::<String>("user"))),
        Some(("ls", s)) => {
            let list_server = s.get_flag("server");
            let paths = s
                .get_many::<String>("path")
                .map(|ps| ps.filter(|p| check_path_exists(p)).collect());
            egress(client::ls(params, paths, list_server))
        }
        Some(("log", _)) => egress(client::log(params)),
        _ => print_subcommand_help("client"),
    }
}
