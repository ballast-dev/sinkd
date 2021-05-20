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

mod barge;

use clap::{Arg, App, SubCommand};
use std::env;




 fn main() {
    let matches = App::new("sinkd")
                        .version("0.1.0")
                        .about("deployable cloud, drop anchor and go")
                        .subcommand(SubCommand::with_name("deploy")
                            .about("deploys anchor point to given IP")
                            .arg(Arg::with_name("IP")
                                .required(true)
                                .help("deploys sinkd daemon on given IP, ssh access required")
                            )
                        )
                        .subcommand(SubCommand::with_name("anchor")
                            .about("anchors folder/file location")
                            .arg(Arg::with_name("FILE")
                                .required(true)
                                .help("sinkd starts watching folder/file")
                            )
                            .help("lets sinkd become `aware` of file or folder location provided")
                        )
                        .subcommand(SubCommand::with_name("start")
                            .about("starts the daemon")
                        )
                        .subcommand(SubCommand::with_name("stop")
                            .about("stops the daemon")
                        )
                        .subcommand(SubCommand::with_name("restart")
                            .about("restarts the daemon")
                        )
                        .get_matches();

// Gets a value for config if supplied by user, or defaults to "default.conf"
//  let config = matches.value_of("config").unwrap_or("default.conf");
//  println!("Value for config: {}", config);

    if let Some(matches) = matches.subcommand_matches("deploy") {
        println!("Using ip address {:?}", matches.value_of("IP"));
        // if matches.value_of() {
        //     println!("Printing debug info...");
        // } else {
        //     println!("Printing normally...");
        // }
    }

    if let Some(matches) = matches.subcommand_matches("anchor") {
        let mut dir = env::current_dir().unwrap();
        println!("cwd = {:?}", dir);
        let this_dir = matches.value_of("FILE").unwrap();
        println!("this dir = {:?}", this_dir);
        dir.push(this_dir);
        println!("Using folder location address {:?}", dir);
    }

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    if let Some(matches) = matches.subcommand_matches("start") {
        // start the daemon
        barge::start_daemon();
    }

    // more program logic goes here...
 }
