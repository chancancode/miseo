//! Host `mise` integration backed by subprocess calls.

use std::process::Command;

use serde::Deserialize;

use crate::{
    error::{Error, invariant},
    fs::{Path, PathBuf},
    spec::{Runtime, RuntimePins, RuntimeSpec, ToolId, ToolSpec},
};

use super::Mise;

#[derive(Debug, Default, Clone, Copy)]
pub struct Cli {
    interactive: bool,
    verbose: u8,
}

impl Cli {
    pub fn new(interactive: bool, verbose: u8) -> Self {
        Self {
            interactive,
            verbose,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new("mise");

        for _ in 0..self.verbose {
            command.arg("-v");
        }

        command
    }

    fn run_capture(&self, args: &[&str]) -> Result<String, Error> {
        let output = self.command().args(args).output()?;

        ensure_success(
            args.join(" "),
            output.status.success(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_status(&self, args: &[String], stream: bool) -> Result<(), Error> {
        if stream {
            self.run_status_streaming(args)
        } else {
            self.run_status_captured(args)
        }
    }

    fn run_status_streaming(&self, args: &[String]) -> Result<(), Error> {
        let status = self.command().args(args).status()?;
        ensure_success(args.join(" "), status.success(), String::new())
    }

    fn run_status_captured(&self, args: &[String]) -> Result<(), Error> {
        let output = self.command().args(args).output()?;
        ensure_success(
            args.join(" "),
            output.status.success(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )
    }
}

fn mise_command_failed(args: String, stderr: String) -> Error {
    Error::MiseCommandFailed {
        command: format!("mise {args}"),
        stderr,
    }
}

fn ensure_success(args: String, ok: bool, stderr: String) -> Result<(), Error> {
    if ok {
        return Ok(());
    }

    Err(mise_command_failed(args, stderr))
}

fn parse_ls_entries(stdout: &str) -> Result<Vec<LsEntry>, Error> {
    Ok(serde_json::from_str(stdout)?)
}

fn install_into_args(
    runtime_versions: &RuntimePins,
    tool_spec: &ToolSpec,
    target_dir: &Path,
) -> Vec<String> {
    let mut args = vec!["exec".to_string()];

    for pin in runtime_versions.values() {
        args.push(pin.runtime_pin());
    }

    args.push("--".to_string());
    args.push("mise".to_string());
    args.push("install-into".to_string());
    args.push(tool_spec.to_string());
    args.push(target_dir.to_string());

    args
}

fn require_non_empty(value: String, context: &str) -> Result<String, Error> {
    if value.is_empty() {
        return Err(invariant!("{context} returned empty output"));
    }

    Ok(value)
}

fn latest_tool_spec(requested: &ToolSpec, version: String) -> Result<ToolSpec, Error> {
    let version = require_non_empty(version, &format!("mise latest for '{requested}'"))?;
    Ok(requested.with_version(&version))
}

fn runtime_spec_from_output(
    runtime: &Runtime,
    selector: String,
    context: &str,
) -> Result<RuntimeSpec, Error> {
    let selector = require_non_empty(selector, context)?;
    Ok(RuntimeSpec::new(runtime.clone(), selector))
}

impl Mise for Cli {
    fn resolve_latest_version(&self, spec: &ToolSpec) -> Result<ToolSpec, Error> {
        let spec_arg = spec.to_string();
        let version = self.run_capture(&["latest", &spec_arg])?;
        latest_tool_spec(spec, version)
    }

    fn resolve_global_runtime_selector(
        &self,
        runtime: &Runtime,
    ) -> Result<Option<RuntimeSpec>, Error> {
        let runtime_name = runtime.as_ref();
        let stdout = self.run_capture(&[
            "ls",
            "--global",
            "--current",
            "--json",
            runtime_name,
            "--cd",
            "/",
        ])?;

        let parsed = parse_ls_entries(&stdout)?;
        parsed
            .into_iter()
            .find_map(|entry| entry.requested_version)
            .map(|selector| runtime_spec_from_output(runtime, selector, "mise ls --global"))
            .transpose()
    }

    fn resolve_global_runtime_installed(&self, runtime: &Runtime) -> Result<bool, Error> {
        let runtime = runtime.as_ref();
        let stdout = self.run_capture(&[
            "ls",
            "--global",
            "--current",
            "--installed",
            "--json",
            runtime,
            "--cd",
            "/",
        ])?;

        let parsed = parse_ls_entries(&stdout)?;
        Ok(!parsed.is_empty())
    }

    fn resolve_current_runtime_version(&self, spec: &RuntimeSpec) -> Result<RuntimeSpec, Error> {
        let runtime = spec.runtime();
        let runtime_name = runtime.as_ref();
        let version = self.run_capture(&[
            "exec",
            &spec.runtime_pin(),
            "--",
            "mise",
            "current",
            runtime_name,
        ])?;
        runtime_spec_from_output(runtime, version, "mise current")
    }

    fn install_into(
        &self,
        runtime_versions: &RuntimePins,
        tool_spec: &ToolSpec,
        target_dir: &Path,
    ) -> Result<(), Error> {
        let args = install_into_args(runtime_versions, tool_spec, target_dir);
        self.run_status(&args, self.interactive)
    }

    fn bin_paths(&self, tool_id: &ToolId, install_dir: &Path) -> Result<Vec<PathBuf>, Error> {
        let path_spec = format!("{tool_id}@path:{install_dir}");
        let stdout = self.run_capture(&["--no-config", "bin-paths", &path_spec])?;
        Ok(stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect())
    }

    fn trust_config(&self, config_path: &Path) -> Result<(), Error> {
        let args = vec!["trust".to_string(), config_path.to_string()];
        self.run_status(&args, false)
    }
}

#[derive(Debug, Deserialize)]
struct LsEntry {
    requested_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::spec::{Runtime, RuntimePins, RuntimeSpec, ToolSpec};

    use super::*;

    #[test]
    fn ensure_success_returns_mise_command_failed_error() {
        let err = ensure_success(
            "install-into npm:prettier".to_string(),
            false,
            "boom".to_string(),
        )
        .unwrap_err();
        match err {
            Error::MiseCommandFailed { command, stderr } => {
                assert_eq!(command, "mise install-into npm:prettier");
                assert_eq!(stderr, "boom");
            }
            _ => panic!("expected mise command failure"),
        }
    }

    #[test]
    fn ensure_success_accepts_ok_status() {
        ensure_success("latest npm:prettier".to_string(), true, String::new()).unwrap();
    }

    #[test]
    fn parse_ls_entries_parses_requested_versions() {
        let parsed =
            parse_ls_entries(r#"[{"requested_version":"22"},{"requested_version":null}]"#).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].requested_version.as_deref(), Some("22"));
        assert_eq!(parsed[1].requested_version.as_deref(), None);
    }

    #[test]
    fn parse_ls_entries_invalid_json_returns_json_error() {
        let err = parse_ls_entries("{").unwrap_err();
        assert!(matches!(err, Error::Json(_)));
    }

    #[test]
    fn install_into_args_builds_expected_exec_command() {
        let mut runtime_versions = RuntimePins::new();
        runtime_versions.insert(
            Runtime::Node,
            RuntimeSpec::new(Runtime::Node, "24.13.1".to_string()),
        );
        runtime_versions.insert(
            Runtime::Python,
            RuntimeSpec::new(Runtime::Python, "3.12.8".to_string()),
        );

        let spec: ToolSpec = "npm:prettier@3.8.1".parse().unwrap();
        let target_dir = PathBuf::from("/tmp/miseo/npm-prettier/3.8.1+node-24.13.1");

        let args = install_into_args(&runtime_versions, &spec, &target_dir);
        assert_eq!(
            args,
            vec![
                "exec",
                "node@24.13.1",
                "python@3.12.8",
                "--",
                "mise",
                "install-into",
                "npm:prettier@3.8.1",
                "/tmp/miseo/npm-prettier/3.8.1+node-24.13.1",
            ]
        );
    }
}
