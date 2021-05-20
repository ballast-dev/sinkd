/**
 * A N C H O R
 *
 * ----
 *
 * Server side of sinkd
 * creates folder and rsync daemon to watch upon
 */
extern crate clap;
extern crate notify;
extern crate regex;
extern crate toml;
#[macro_use]
extern crate serde_derive;


// use clap::{Arg, App, SubCommand};
use std::env;
use std::path::Path;
use std::process::exit as exit;
use regex::Regex;

mod cli;
mod daemon;
mod defs;

#[allow(dead_code)]
fn main() {
    let matches = cli::build_cli().get_matches();
            
    // if let Some(matches) = matches.subcommand_matches("deploy") {
        
    //     let ip = matches.value_of("IP").unwrap_or("localhost");
    //     let valid_ip = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap(); 

    //     if valid_ip.is_match(ip) {
    //         cli::deploy(ip);
    //     } else {
    //         println!("'{}' Invalid IP address", ip);
    //         exit(1);
    //     }
    // }

    if let Some(matches) = matches.subcommand_matches("add") {
        let path = String::from(matches.value_of("PATH").unwrap());
        println!("adding file!");
        
        if Path::new(&path[..]).exists() {
            cli::add(cli::DaemonType::Barge, path); // always a Barge from cli
        } else {
            println!("'{}' does not exist", path);
        }
    }
    
    // if let Some(matches) = matches.subcommand_matches("deploy") {
    //     cli::underway(cli::DaemonType::Barge);
    // }

    if let Some(matches) = matches.subcommand_matches("ls") {
        cli::list();
    }

    if let Some(matches) = matches.subcommand_matches("rm") {
        cli::remove();
    }

    if let Some(matches) = matches.subcommand_matches("stop") {
        cli::stop();
    }

    if let Some(matches) = matches.subcommand_matches("adduser") {
        cli::adduser(matches.values_of("USER").unwrap().collect());
    }


}
