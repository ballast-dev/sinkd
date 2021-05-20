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
        let this_dir = String::from(matches.value_of("FILE").unwrap());
        println!("adding file!");
        
        if Path::new(&this_dir[..]).exists() {
            cli::anchor(cli::DaemonType::Barge, this_dir); // always a Barge from cli
        } else {
            println!("'{}' does not exist", this_dir);
        }
    }
    
    if let Some(matches) = matches.subcommand_matches("deploy") {
        cli::underway(cli::DaemonType::Barge);
    }

    if let Some(matches) = matches.subcommand_matches("add") {
        cli::add();
    }

    if let Some(matches) = matches.subcommand_matches("list") {
        cli::list();
    }

    if let Some(matches) = matches.subcommand_matches("remove") {
        cli::remove();
    }

    if let Some(matches) = matches.subcommand_matches("stop") {
        cli::stop();
    }

    if let Some(matches) = matches.subcommand_matches("recruit") {
        cli::recruit(matches.values_of("USER").unwrap().collect());
    }


}
