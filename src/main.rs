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
                            .about("adds folder/file to watch list")
                            .arg(Arg::with_name("FILE")
                                .required(true)
                                .help("sinkd starts watching folder/file")
                            )
                            .help("usage: sinkd anchor [OPTION] FILE\n\
                                   lets sinkd become `aware` of file or folder location provided")
                        )
                        .subcommand(SubCommand::with_name("parley")
                            // really nice printout
                            .about("list watched dirs")
                        )
                        .subcommand(SubCommand::with_name("brig")
                            .about("removes PATH from list of watched directories")
                            .help("usage: sinkd brig PATH")
                        )
                        .subcommand(SubCommand::with_name("underway")
                            .about("starts local daemon to watch over files|folders")
                            .help("no option necessary, spawns daemon locally")
                        )
                        .subcommand(SubCommand::with_name("snag")
                            .about("stops the sinkd daemon")
                        )
                        .subcommand(SubCommand::with_name("oilskins")
                            .about("stops and starts the daemon (updates config)")
                        )
                        .subcommand(SubCommand::with_name("recruit")
                            .about("add user to watch")
                            .help("sinkd recruit USER DIRECTORY")
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
        println!("anchors away!");
        let this_dir = matches.value_of("FILE").unwrap();
        
        if Path::new(this_dir).exists() {
            cli::anchor(cli::DaemonType::Barge, this_dir); // always a Barge from cli
        } else {
            println!("'{}' does not exist", this_dir);
        }
    }
    
    if let Some(matches) = matches.subcommand_matches("underway") {
        cli::underway(cli::DaemonType::Barge);
    }

    if let Some(matches) = matches.subcommand_matches("snag") {
        cli::snag();
    }

    if let Some(matches) = matches.subcommand_matches("oilskins") {
        cli::oilskins();
    }

}
