use crate::{
    cli::output::Ui,
    error::Error,
    mise::Mise,
    spec::{RuntimeSpec, ToolId},
    tasks,
    workspace::Workspace,
};
use clap::ArgAction;

use super::Command;

#[derive(Debug, clap::Args, Clone)]
pub struct Upgrade {
    /// Installed tool id: <backend>:<name> (version suffix is not accepted).
    pub tool_spec: ToolId,

    /// Runtime selector(s): <runtime@selector> (repeatable).
    #[arg(long = "use", value_name = "RUNTIME@SELECTOR", action = ArgAction::Append)]
    pub uses: Vec<RuntimeSpec>,
}

impl Command for Upgrade {
    fn execute(
        self,
        workspace: &mut Workspace,
        mise: &impl Mise,
        ui: &Ui<'_>,
    ) -> Result<(), Error> {
        match tasks::upgrade::execute(mise, workspace, self.tool_spec, self.uses.try_into()?)? {
            Ok(success) => ui.upgrade_success(&success),
            Err(already_current) => ui.upgrade_already_current(&already_current),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cli::{Cli, Commands, app::TestApp},
        error::Error,
        spec::Runtime,
    };
    use clap::Parser;

    use super::*;

    fn cli_command(input: &str) -> Commands {
        Cli::try_parse_from(input.split_whitespace())
            .unwrap()
            .command
    }

    fn command(input: &str) -> Upgrade {
        let Commands::Upgrade(args) = cli_command(input) else {
            panic!("wrong command");
        };

        args
    }

    #[test]
    fn parses_upgrade() {
        let args = command("miseo upgrade npm:http-server");
        assert_eq!(args.tool_spec.to_string(), "npm:http-server");
        assert!(args.uses.is_empty());
    }

    #[test]
    fn parses_upgrade_with_repeatable_use() {
        let args = command("miseo upgrade npm:http-server --use node@22 --use python@3.12");
        assert_eq!(args.tool_spec.to_string(), "npm:http-server");
        assert_eq!(
            args.uses,
            vec![
                RuntimeSpec::new(Runtime::Node, "22"),
                RuntimeSpec::new(Runtime::Python, "3.12"),
            ]
        );
    }

    #[test]
    fn rejects_versioned_tool_id_for_upgrade() {
        assert!(Cli::try_parse_from(["miseo", "upgrade", "npm:prettier@3"]).is_err());
    }

    #[test]
    fn upgrade_rejects_duplicate_use_runtime() {
        let test = TestApp::new();
        let err = test
            .run(command(
                "miseo upgrade npm:prettier --use node@22 --use node@lts",
            ))
            .unwrap_err();

        assert!(matches!(err, Error::DuplicateRuntimeUse { runtime } if runtime == "node"));
    }

    #[test]
    fn upgrade_installs_latest_and_prunes_old_variants() {
        let mut test = TestApp::new();
        test.mise_mut()
            .use_g(Runtime::Node, "24", "24.13.1")
            .current(Runtime::Node, "24.13.1", "24.13.1")
            .register_package("npm:prettier", "3.8.1", ["prettier"]);

        test.run(cli_command("miseo install npm:prettier")).unwrap();
        test.out().clear();

        test.mise_mut()
            .register_package("npm:prettier", "3.8.2", ["prettier"]);

        test.run(command("miseo upgrade npm:prettier")).unwrap();

        let fs = test.fs();
        assert!(
            !fs.exists(&test.path("npm-prettier/3.8.1+node-24.13.1"))
                .unwrap()
        );
        assert_eq!(
            fs.readlink(&test.path("npm-prettier/current")).unwrap(),
            Some("3.8.2+node-24.13.1".into())
        );

        test.out().assert_out(&[
            "miseo upgraded npm:prettier@3.8.2 [node@24.13.1] in ...ms",
            "  miseo installed command: prettier",
            "  miseo removed old variants: 3.8.1+node-24.13.1",
        ]);

        let manifest = test.manifest();
        assert_eq!(
            manifest.current_variant_str("npm:prettier"),
            Some("3.8.2+node-24.13.1")
        );
        assert_eq!(manifest.variant_count_str("npm:prettier"), 1);
        assert_eq!(manifest.command_owner_str("prettier"), Some("npm:prettier"));
    }

    #[test]
    fn upgrade_with_use_repins_runtime_from_selector() {
        let mut test = TestApp::new();
        test.mise_mut()
            .use_g(Runtime::Node, "22", "22.13.1")
            .register_package("npm:prettier", "3.8.1", ["prettier"]);

        test.run(cli_command("miseo install npm:prettier")).unwrap();
        test.out().clear();

        test.mise_mut()
            .current(Runtime::Node, "lts", "24.13.1")
            .register_package("npm:prettier", "3.8.2", ["prettier"]);

        test.run(command("miseo upgrade npm:prettier --use node@lts"))
            .unwrap();

        let fs = test.fs();
        assert!(
            !fs.exists(&test.path("npm-prettier/3.8.1+node-22.13.1"))
                .unwrap()
        );
        assert_eq!(
            fs.readlink(&test.path("npm-prettier/current")).unwrap(),
            Some("3.8.2+node-24.13.1".into())
        );

        test.out().assert_out(&[
            "miseo upgraded npm:prettier@3.8.2 [node@24.13.1] in ...ms",
            "  miseo installed command: prettier",
            "  miseo removed old variants: 3.8.1+node-22.13.1",
        ]);

        let mise_toml = test.path("npm-prettier/3.8.2+node-24.13.1/mise.toml");
        let content = fs.read_file(&mise_toml).unwrap();
        assert_eq!(content, "[tools]\nnode = \"24.13.1\"\n");

        let manifest = test.manifest();
        assert_eq!(
            manifest.current_variant_str("npm:prettier"),
            Some("3.8.2+node-24.13.1")
        );
        assert_eq!(manifest.variant_count_str("npm:prettier"), 1);
        assert_eq!(manifest.command_owner_str("prettier"), Some("npm:prettier"));
    }

    #[test]
    fn upgrade_returns_already_current_when_no_change() {
        let mut test = TestApp::new();
        test.mise_mut()
            .latest("npm:prettier@latest", "3.8.2")
            .use_g(Runtime::Node, "24", "24.13.1")
            .current(Runtime::Node, "24.13.1", "24.13.1")
            .register_package("npm:prettier", "3.8.2", ["prettier"]);

        test.run(cli_command("miseo install npm:prettier")).unwrap();
        test.out().clear();

        test.run(command("miseo upgrade npm:prettier")).unwrap();
        test.out()
            .assert_out(&["miseo npm:prettier@3.8.2 [node@24.13.1] already current"]);

        let manifest = test.manifest();
        assert_eq!(
            manifest.current_variant_str("npm:prettier"),
            Some("3.8.2+node-24.13.1")
        );
        assert_eq!(manifest.variant_count_str("npm:prettier"), 1);
    }

    #[test]
    fn upgrade_errors_when_tool_is_not_installed() {
        let test = TestApp::new();
        let err = test.run(command("miseo upgrade npm:prettier")).unwrap_err();

        assert!(matches!(err, Error::ToolNotInstalled { .. }));
    }
}
