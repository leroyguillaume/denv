// IMPORTS

use ::log::error;
use clap::Parser;
use cli::Cli;
use run::{Error, Runner};
use std::process;

// MODS

mod archive;
mod cfg;
mod cli;
mod fs;
mod log;
mod net;
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
    let exit_code = match runner.run(cli.cmd, cli.opts) {
        Ok(()) => 0,
        Err(err) => match &err {
            Error::Compute(errs) => {
                error!("{}", err);
                for err in errs {
                    error!("{}: {}", err.var.name(), err.cause);
                }
                exitcode::SOFTWARE
            }
            Error::Config(err) => {
                error!("Unable to load configuration");
                match err {
                    cfg::Error::Config(errs) => {
                        for err in errs {
                            error!("{}", err);
                        }
                    }
                    err => error!("{}", err),
                }
                exitcode::CONFIG
            }
            Error::Install(errs) => {
                error!("{}", err);
                for err in errs {
                    error!("{}: {}", err.soft.name(), err.cause);
                }
                exitcode::SOFTWARE
            }
            err => {
                error!("{}", err);
                exitcode::SOFTWARE
            }
        },
    };
    process::exit(exit_code);
}
