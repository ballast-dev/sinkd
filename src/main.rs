use log::{debug, info};
use std::process::ExitCode;

use sinkd::cli;
use sinkd::parameters;
use sinkd::shiplog;
use sinkd::time;

fn windoze() -> ExitCode {
    let cli_cmd = cli::build().no_binary_name(true);
    let matches = match cli_cmd.try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };
    let params = match parameters::DaemonParameters::from_matches(&matches) {
        Ok(p) => p,
        Err(e) => return cli::egress::<()>(sinkd::bad!(e)),
    };
    if let Err(e) = shiplog::init(params.shared()) {
        let _ = std::fs::write("sinkd_error.log", format!("{e:?}"));
    }
    info!("-- windoze --");
    match matches.subcommand() {
        Some(("client", _)) => debug!("windows client!"),
        Some(("server", _)) => debug!("windows server!"),
        _ => debug!("uh oh... matches>> {matches:?}"),
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--windows-daemon") {
        return windoze();
    }

    let cmd = cli::build();
    let matches = match cmd.try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };

    let params = match parameters::DaemonParameters::from_matches(&matches) {
        Ok(p) => p,
        Err(e) => return cli::egress::<()>(sinkd::bad!(e)),
    };

    if params.shared().debug > 0 {
        println!("timestamp {}", time::stamp(None));
    }

    match (matches.subcommand(), &params) {
        (Some(("server", sub)), parameters::DaemonParameters::Server(s)) => {
            cli::server::dispatch(sub, s)
        }
        (Some(("client", sub)), parameters::DaemonParameters::Client(c)) => {
            cli::client::dispatch(sub, c)
        }
        _ => unreachable!("clap subcommand_required aligns with DaemonParameters::from_matches"),
    }
}
