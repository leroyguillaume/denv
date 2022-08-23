// IMPORTS

use clap::Parser;
use cli::Cli;
use run::Runner;

// MODS

mod cfg;
mod cli;
mod fs;
mod log;
mod run;
mod soft;
#[cfg(test)]
mod test;
mod var;

// FUNTIONS

fn main() {
    let cli = Cli::parse();
    let log_level = cli.opts.verbosity.to_log_level();
    log::Logger::init(log_level, !cli.opts.no_color).unwrap();
    let runner = Runner::default();
    runner.run(cli.cmd, cli.opts).unwrap();
}
