//! High-level CLI UI renderer: phrasing, color, and line-shape decisions.

use owo_colors::OwoColorize;

use crate::{
    error::Error,
    spec::{ToolId, ToolSpec},
    tasks::{install, uninstall, upgrade},
};

use super::Output;

pub struct Ui<'a> {
    out: &'a dyn Output,
    concise: bool,
    color: bool,
}

impl<'a> Ui<'a> {
    pub fn new(out: &'a dyn Output, concise: bool) -> Self {
        Self {
            out,
            concise,
            color: out.is_tty() && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    #[cfg(test)]
    pub(in crate::cli) fn is_concise(&self) -> bool {
        self.concise
    }

    fn line(&self, line: impl AsRef<str>) {
        self.out.println(line.as_ref());
    }

    fn err_line(&self, line: impl AsRef<str>) {
        self.out.eprintln(line.as_ref());
    }

    fn brand(&self) -> String {
        green("miseo", self.color)
    }

    fn highlight(&self, text: &str) -> String {
        blue(text, self.color)
    }

    fn comment(&self, text: &str) -> String {
        dim(text, self.color)
    }

    fn hint_label(&self) -> String {
        yellow("hint:", self.color)
    }

    fn error_label(&self) -> String {
        red("error:", self.color)
    }

    fn hint(&self, body: impl AsRef<str>) {
        self.line(format!("  {} {}", self.hint_label(), body.as_ref()));
    }

    fn error_message(&self, body: impl AsRef<str>) {
        self.err_line(format!("{} {}", self.error_label(), body.as_ref()));
    }

    fn error_hint(&self, body: impl AsRef<str>) {
        self.err_line(format!("{} {}", self.hint_label(), body.as_ref()));
    }

    fn join_commands(&self, commands: &[String]) -> String {
        commands
            .iter()
            .map(|command| self.highlight(command))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Render a successful install.
    pub fn install_success(&self, success: &install::Success) {
        let runtime_suffix = runtime_suffix(&success.runtimes);
        let spec = format!("{}@{}", success.tool_id, success.package_version);

        if self.concise {
            self.installed_concise(
                &spec,
                &runtime_suffix,
                success.action.as_str(),
                &success.commands,
                success.elapsed_ms,
            );
        } else {
            self.installed_verbose(
                &spec,
                &runtime_suffix,
                success.action.as_str(),
                &success.commands,
                success.elapsed_ms,
            );
        }
    }

    /// Render an already-current install result.
    pub fn install_already_current(&self, current: &install::AlreadyCurrent) {
        let runtime_suffix = runtime_suffix(&current.runtimes);
        let spec = format!("{}@{}", current.tool_id, current.package_version);

        if self.concise {
            self.already_current_concise(&spec, &runtime_suffix);
        } else {
            self.already_current_verbose(&spec, &runtime_suffix);
        }
    }

    /// Render follow-up install hints when no action was taken.
    pub fn install_hints(&self, tool_spec: &ToolSpec, tool_id: &ToolId) {
        if self.concise {
            return;
        }
        self.hint(format!(
            "miseo install {} --force    {}",
            tool_spec,
            self.comment("# reinstall current variant")
        ));
        self.hint(format!(
            "miseo upgrade {}            {}",
            tool_id,
            self.comment("# upgrade to newer version")
        ));
    }

    /// Render a successful upgrade.
    pub fn upgrade_success(&self, success: &upgrade::Success) {
        let runtime_suffix = runtime_suffix(&success.runtimes);
        let spec = format!("{}@{}", success.tool_id, success.package_version);

        if self.concise {
            self.upgraded_concise(
                &spec,
                &runtime_suffix,
                &success.commands,
                success.elapsed_ms,
                &success.removed_variants,
            );
        } else {
            self.upgraded_verbose(
                &spec,
                &runtime_suffix,
                &success.commands,
                success.elapsed_ms,
                &success.removed_variants,
            );
        }
    }

    /// Render an already-current upgrade result.
    pub fn upgrade_already_current(&self, current: &upgrade::AlreadyCurrent) {
        let runtime_suffix = runtime_suffix(&current.runtimes);
        let spec = format!("{}@{}", current.tool_id, current.package_version);

        if self.concise {
            self.already_current_concise(&spec, &runtime_suffix);
        } else {
            self.already_current_verbose(&spec, &runtime_suffix);
        }
    }

    /// Render a successful uninstall.
    pub fn uninstalled(&self, view: &uninstall::Outcome) {
        if self.concise {
            self.uninstalled_concise(&view.tool_id, &view.removed_commands);
        } else {
            self.uninstalled_verbose(&view.tool_id, &view.removed_commands);
        }
    }

    /// Render one error with optional tailored hints.
    pub fn error(&self, err: &Error) {
        match err {
            Error::ToolNotInstalledOrphanFound { tool_id, path } => {
                self.error_message(format!("tool {} is not installed", self.highlight(tool_id)));
                self.error_hint(format!(
                    "found unmanaged install at {}; run miseo uninstall {} --force to clean it up",
                    self.highlight(path),
                    self.highlight(tool_id)
                ));
            }
            _ => self.error_message(err.to_string()),
        }
    }

    fn installed_concise(
        &self,
        spec: &str,
        runtime_suffix: &str,
        action: &str,
        commands: &[String],
        elapsed_ms: u128,
    ) {
        self.line(format!(
            "{} {} {}{} in {}ms",
            self.brand(),
            action,
            self.highlight(spec),
            runtime_suffix,
            elapsed_ms
        ));

        self.line(format!(
            "{} {}: {}",
            self.brand(),
            install_label(commands.len()),
            self.join_commands(commands)
        ));
    }

    fn installed_verbose(
        &self,
        spec: &str,
        runtime_suffix: &str,
        action: &str,
        commands: &[String],
        elapsed_ms: u128,
    ) {
        self.line(format!(
            "miseo {action} {spec}{runtime_suffix} in {elapsed_ms}ms"
        ));

        self.line(format!(
            "  miseo {}: {}",
            install_label(commands.len()),
            commands.join(", ")
        ));
    }

    fn already_current_concise(&self, spec: &str, runtime_suffix: &str) {
        self.line(format!(
            "{} {}{} already current",
            self.brand(),
            self.highlight(spec),
            runtime_suffix
        ));
    }

    fn already_current_verbose(&self, spec: &str, runtime_suffix: &str) {
        self.line(format!("miseo {spec}{runtime_suffix} already current"));
    }

    fn upgraded_concise(
        &self,
        spec: &str,
        runtime_suffix: &str,
        commands: &[String],
        elapsed_ms: u128,
        removed_variants: &[String],
    ) {
        self.line(format!(
            "{} upgraded {}{} in {}ms",
            self.brand(),
            self.highlight(spec),
            runtime_suffix,
            elapsed_ms
        ));

        self.line(format!(
            "{} {}: {}",
            self.brand(),
            install_label(commands.len()),
            self.join_commands(commands)
        ));

        if !removed_variants.is_empty() {
            self.line(format!(
                "{} removed old variants: {}",
                self.brand(),
                removed_variants.join(", ")
            ));
        }
    }

    fn upgraded_verbose(
        &self,
        spec: &str,
        runtime_suffix: &str,
        commands: &[String],
        elapsed_ms: u128,
        removed_variants: &[String],
    ) {
        self.line(format!(
            "miseo upgraded {spec}{runtime_suffix} in {elapsed_ms}ms"
        ));

        self.line(format!(
            "  miseo {}: {}",
            install_label(commands.len()),
            commands.join(", ")
        ));

        if !removed_variants.is_empty() {
            self.line(format!(
                "  miseo removed old variants: {}",
                removed_variants.join(", ")
            ));
        }
    }

    fn uninstalled_concise(&self, tool_id: &ToolId, removed_commands: &[String]) {
        self.line(format!(
            "{} uninstalled {}",
            self.brand(),
            self.highlight(&tool_id.to_string())
        ));

        if !removed_commands.is_empty() {
            self.line(format!(
                "{} {}: {}",
                self.brand(),
                remove_label(removed_commands.len()),
                self.join_commands(removed_commands)
            ));
        }
    }

    fn uninstalled_verbose(&self, tool_id: &ToolId, removed_commands: &[String]) {
        self.line(format!("miseo uninstalled {tool_id}"));

        if !removed_commands.is_empty() {
            self.line(format!(
                "  miseo {}: {}",
                remove_label(removed_commands.len()),
                removed_commands.join(", ")
            ));
        }
    }
}

fn runtime_suffix(runtimes: &[String]) -> String {
    if runtimes.is_empty() {
        String::new()
    } else {
        format!(" [{}]", runtimes.join(", "))
    }
}

fn install_label(count: usize) -> &'static str {
    if count == 1 {
        "installed command"
    } else {
        "installed commands"
    }
}

fn remove_label(count: usize) -> &'static str {
    if count == 1 {
        "removed command"
    } else {
        "removed commands"
    }
}

fn green(text: &str, enabled: bool) -> String {
    if enabled {
        text.green().to_string()
    } else {
        text.to_string()
    }
}

fn blue(text: &str, enabled: bool) -> String {
    if enabled {
        text.blue().to_string()
    } else {
        text.to_string()
    }
}

fn yellow(text: &str, enabled: bool) -> String {
    if enabled {
        text.yellow().to_string()
    } else {
        text.to_string()
    }
}

fn red(text: &str, enabled: bool) -> String {
    if enabled {
        text.red().to_string()
    } else {
        text.to_string()
    }
}

fn dim(text: &str, enabled: bool) -> String {
    if enabled {
        text.dimmed().to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli::output::test::Test, tasks::install};

    #[test]
    fn installed_verbose_renders_expected_lines() {
        let out = Test::new(false);
        let ui = Ui::new(&out, false);
        ui.install_success(&install::Success {
            action: install::Action::Installed,
            tool_id: "npm:prettier".parse().unwrap(),
            package_version: "3.8.1".to_string(),
            runtimes: vec!["node@24.13.1".to_string()],
            commands: vec!["prettier".to_string()],
            elapsed_ms: 150,
        });

        out.assert_out(&[
            "miseo installed npm:prettier@3.8.1 [node@24.13.1] in 150ms",
            "  miseo installed command: prettier",
        ]);
    }

    #[test]
    fn installed_concise_renders_expected_lines_without_color_when_not_tty() {
        let out = Test::new(false);
        let ui = Ui::new(&out, true);
        ui.install_success(&install::Success {
            action: install::Action::Installed,
            tool_id: "npm:prettier".parse().unwrap(),
            package_version: "3.8.1".to_string(),
            runtimes: vec![],
            commands: vec!["prettier".to_string(), "prettierd".to_string()],
            elapsed_ms: 42,
        });

        out.assert_out(&[
            "miseo installed npm:prettier@3.8.1 in 42ms",
            "miseo installed commands: prettier, prettierd",
        ]);
    }

    #[test]
    fn error_orphan_is_human_readable() {
        let out = Test::new(false);
        let ui = Ui::new(&out, false);
        ui.error(&Error::ToolNotInstalledOrphanFound {
            tool_id: "gem:webrick".to_string(),
            path: "/tmp/miseo/gem-webrick".to_string(),
        });

        out.assert_err(&[
            "error: tool gem:webrick is not installed",
            "hint: found unmanaged install at /tmp/miseo/gem-webrick; \
             run miseo uninstall gem:webrick --force to clean it up",
        ]);
    }
}
