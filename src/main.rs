// IMPORTS

use clap::Parser;
use cli::Cli;

// MODS

mod cli;
mod log;

// FUNTIONS

fn main() {
    let cli = Cli::parse();
    let log_level = cli.opts.verbosity.to_log_level();
    log::Logger::init(log_level, !cli.opts.no_color).unwrap();
}
