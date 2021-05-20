/**
 * A N C H O R
 *
 * ----
 *
 * Server side of sinkd
 * creates folder and rsync daemon to watch upon
 */
 // (Full example with detailed comments in examples/01b_quick_example.rs)
 //
 // This example demonstrates clap's full 'builder pattern' style of creating arguments which is
 // more verbose, but allows easier editing, and at times more advanced options, or the possibility
 // to generate arguments dynamically.
extern crate clap;
extern crate notify;
extern crate yaml_rust;
extern crate regex;


use clap::{Arg, App, SubCommand};
use std::env;
use std::path::Path;
use std::process::exit as exit;
use regex::Regex;

mod cli;
mod daemon;

#[allow(dead_code)]
fn main() {
    let matches = App::new("sinkd")
                        .version("0.1.0")
                        .about("deployable cloud, drop anchor and go")
                        .subcommand(SubCommand::with_name("deploy")
                            .about("deploys sinkd to given IP")
                            .arg(Arg::with_name("IP")
                                .required(true)
                                .help("IPv4 address, ssh access required")
                            )
                            .help("sets up sinkd server on remote computer")
                        )
                        .subcommand(SubCommand::with_name("anchor")
                            .about("anchors folder/file location")
                            .arg(Arg::with_name("LOCATION")
                                .required(true)
                                .help("sinkd starts watching folder/file")
                            )
                            .help("usage: sinkd anchor [OPTION] FILE\n\
                                   lets sinkd become `aware` of file or folder location provided")
                        )
                        .subcommand(SubCommand::with_name("start")
                            .about("starts the daemon")
                        )
                        .subcommand(SubCommand::with_name("stop")
                            .about("stops the daemon")
                        )
                        .subcommand(SubCommand::with_name("restart")
                            .about("stops and starts the daemon (update config)")
                        )
                        .get_matches();

// Gets a value for config if supplied by user, or defaults to "default.conf"
//  let config = matches.value_of("config").unwrap_or("default.conf");
//  println!("Value for config: {}", config);

    if let Some(matches) = matches.subcommand_matches("deploy") {
        
        let ip = matches.value_of("IP").unwrap();
        let valid_ip = Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap(); 

        if valid_ip.is_match(ip) {
            cli::deploy(ip);
        } else {
            println!("'{}' Invalid IP address", ip);
            exit(1);
        }
    }

    if let Some(matches) = matches.subcommand_matches("anchor") {
        
        let this_dir = matches.value_of("FILE").unwrap();
        
        if Path::new(this_dir).exists() {
            cli::anchor(this_dir);
        } else {
            println!("'{}' does not exist", this_dir);
        }
    }
    
    if let Some(matches) = matches.subcommand_matches("start") {
        cli::start();
    }

    if let Some(matches) = matches.subcommand_matches("stop") {
        cli::stop();
    }

    if let Some(matches) = matches.subcommand_matches("restart") {
        cli::restart();
    }

}