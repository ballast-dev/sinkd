//extern crate clap;

use clap::Shell;

include!("src/main.rs");

fn main() {
    let mut sinkd = build_cli();
    sinkd.gen_completions("sinkd", Shell::Zsh, env!("OUT_DIR"));
}
