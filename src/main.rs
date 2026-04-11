use clap::ArgMatches;
use log::{debug, info};
use std::process::ExitCode;

mod cli;
mod parameters;
mod time;
mod outcome;
mod shiplog;
mod server;
mod client;
mod ipc;
mod config;
mod conflict;
mod rsync;
mod test_hooks;

fn windoze() -> ExitCode {
    let cli = cli::build().no_binary_name(true);
    let matches = cli.get_matches();
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

    let cli = build_sinkd();
    let matches = match cli.try_get_matches() {
        Ok(m) => m,
        Err(e) => e.exit(),
    };

    println!("timestamp {}", time::stamp(None));

    let params = match DaemonParameters::from_matches(matches) {
        Ok(p) => p,
        Err(e) => return egress::<String>(bad!(e)),
    };

    match (matches.subcommand(), &params) {
        (Some(("server", sub)), DaemonParameters::Server(s)) => server::dispatch(sub, s),
        (Some(("client", sub)), DaemonParameters::Client(c)) => client::dispatch(sub, c),
        _ => {
            fancy_error!("unknown subcommand");
            cli.print_help();
            ExitCode::FAILURE
        }
    }
}
