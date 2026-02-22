//! CLI subcommands and the shared command execution trait.

use clap::Subcommand;

use crate::{cli::output::Ui, error::Error, mise::Mise, workspace::Workspace};

pub mod install;
pub mod uninstall;
pub mod upgrade;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Install a tool into ~/.miseo and export its command(s) into ~/.miseo/.bin.
    Install(install::Install),
    /// Upgrade an installed tool to a newer resolved package version.
    Upgrade(upgrade::Upgrade),
    /// Uninstall a tool from ~/.miseo and remove exported command(s) from ~/.miseo/.bin.
    #[command(alias = "remove")]
    Uninstall(uninstall::Uninstall),
}

pub trait Command {
    fn execute(self, workspace: &mut Workspace, mise: &impl Mise, ui: &Ui<'_>)
    -> Result<(), Error>;
}

impl Command for Commands {
    fn execute(
        self,
        workspace: &mut Workspace,
        mise: &impl Mise,
        ui: &Ui<'_>,
    ) -> Result<(), Error> {
        match self {
            Commands::Install(args) => args.execute(workspace, mise, ui),
            Commands::Upgrade(args) => args.execute(workspace, mise, ui),
            Commands::Uninstall(args) => args.execute(workspace, mise, ui),
        }
    }
}
