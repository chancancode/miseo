//! CLI entrypoint types plus command/output modules.

use clap::{ArgAction, Parser};

mod app;
mod commands;
mod output;

use commands::Commands;

const LONG_ABOUT: &str = "Stable global CLI tools on top of mise.\n\n\
Each tool is installed into ~/.miseo with explicit runtime versions, and \
exported via ~/.miseo/.bin.";

#[derive(Debug, Parser)]
#[command(name = "miseo")]
#[command(about = "Stable global CLI tools on top of mise")]
#[command(long_about = LONG_ABOUT)]
pub struct Cli {
    /// Increase output verbosity (`-v`, `-vv`, ...).
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

pub use app::App;

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_global_verbose() {
        let cli = Cli::try_parse_from(["miseo", "-vv", "install", "npm:prettier"]).unwrap();

        let Commands::Install(args) = cli.command else {
            panic!("wrong command");
        };

        assert_eq!(cli.verbose, 2);
        assert_eq!(args.tool_spec.to_string(), "npm:prettier");
    }
}
