use log::{debug, info};
use std::process::ExitCode;

use sinkd::cli;
use sinkd::parameters;
use sinkd::shiplog;
use sinkd::time;

fn windoze() -> ExitCode {
    let cli_cmd = cli::build().no_binary_name(true);
    let matches = cli_cmd.get_matches();
    let params = parameters::DaemonParameters::from_matches(&matches).unwrap();
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

    println!("timestamp {}", time::stamp(None));

    let params = match parameters::DaemonParameters::from_matches(&matches) {
        Ok(p) => p,
        Err(e) => return cli::egress::<()>(sinkd::bad!(e)),
    };

    match (matches.subcommand(), &params) {
        (Some(("server", sub)), parameters::DaemonParameters::Server(s)) => {
            cli::server::dispatch(sub, s)
        }
        (Some(("client", sub)), parameters::DaemonParameters::Client(c)) => {
            cli::client::dispatch(sub, c)
        }
        _ => {
            sinkd::fancy_error!("unknown subcommand");
            ExitCode::FAILURE
        }
    }
}
