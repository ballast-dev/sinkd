extern crate clap;
extern crate notify;
extern crate regex;
extern crate toml;
#[macro_use]
extern crate serde_derive;


use std::path::Path;
// use std::process::exit as exit;

mod cli;
mod daemon;
mod defs;

#[allow(dead_code)]
fn main() {
    let matches = cli::build_cli().get_matches();
    
    if let Some(matches) = matches.subcommand_matches("add") {
        let path = String::from(matches.value_of("PATH").unwrap());
        println!("adding file!");
        
        if Path::new(&path[..]).exists() {
            cli::add(cli::DaemonType::Barge, path); // always a Barge from cli
        } else {
            println!("'{}' does not exist", path);
        }
    }
    
    if let Some(matches) = matches.subcommand_matches("adduser") {
        cli::adduser(matches.values_of("USER").unwrap().collect());
    }

    if let Some(_) = matches.subcommand_matches("ls") {
        cli::list();
    }

    if let Some(_) = matches.subcommand_matches("rm") {
        cli::remove();
    }

    if let Some(_) = matches.subcommand_matches("start") {
        cli::start();
    }

    if let Some(_) = matches.subcommand_matches("stop") {
        cli::stop();
    }
    
    if let Some(_) = matches.subcommand_matches("restart") {
        cli::restart();
    }

    if matches.is_present("daemon") {
        cli::daemon();
    }

}
