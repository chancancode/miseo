//! Error types, exit-code mapping, and invariant helpers.

use std::io;

use thiserror::Error;

#[macro_export]
macro_rules! invariant {
    ($($arg:tt)*) => {
        $crate::error::Error::ManifestInvariant(format!($($arg)*))
    };
}

pub use crate::invariant;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid tool spec '{spec}': {reason}")]
    InvalidToolSpec { spec: String, reason: String },

    #[error("invalid --use value '{input}': expected <runtime@selector>")]
    InvalidRuntimeSelector { input: String },

    #[error(
        "runtime '{runtime}' was provided more than once in --use; specify each runtime only once"
    )]
    DuplicateRuntimeUse { runtime: String },

    #[error(
        "can't infer a runtime to use for backend '{backend}'; \
         select the runtime(s) explicitly with --use (for example `--use node@lts`)"
    )]
    UnmappedBackendWithoutUse { backend: String },

    #[error(
        "no global version is configured for runtime '{runtime}'; \
         configure one in mise with `mise use -g {runtime}`, or pass --use \
         to select the runtime(s) explicitly for this install (for example `--use node@lts`)"
    )]
    MissingGlobalRuntime { runtime: String },

    #[error(
        "global '{runtime}' runtime is configured but not installed; \
         install it with `mise use -g {runtime}`, or pass --use \
         to select the runtime(s) explicitly for this install (for example `--use node@lts`)"
    )]
    RuntimeNotInstalled { runtime: String },

    #[error(
        "cannot install '{requested}': command '{command}' is already linked to '{owner}'; \
         run `miseo uninstall {owner}` first"
    )]
    CommandOwnershipConflict {
        command: String,
        owner: String,
        requested: String,
    },

    #[error("tool '{tool_id}' is not installed")]
    ToolNotInstalled { tool_id: String },

    #[error("tool '{tool_id}' is not installed (found unmanaged install at '{path}')")]
    ToolNotInstalledOrphanFound { tool_id: String, path: String },

    #[error("internal state error: {0}")]
    ManifestInvariant(String),

    #[error(
        "failed to run `{command}`: {}",
        if stderr.is_empty() { "<no stderr output>" } else { stderr }
    )]
    MiseCommandFailed { command: String, stderr: String },

    #[error("internal I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("internal TOML parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("internal TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("internal JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidToolSpec { .. }
            | Self::InvalidRuntimeSelector { .. }
            | Self::DuplicateRuntimeUse { .. } => 2,
            Self::UnmappedBackendWithoutUse { .. }
            | Self::MissingGlobalRuntime { .. }
            | Self::RuntimeNotInstalled { .. }
            | Self::CommandOwnershipConflict { .. }
            | Self::ToolNotInstalled { .. }
            | Self::ToolNotInstalledOrphanFound { .. } => 3,
            Self::ManifestInvariant(_)
            | Self::MiseCommandFailed { .. }
            | Self::Io(_)
            | Self::TomlDe(_)
            | Self::TomlSer(_)
            | Self::Json(_) => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_macro_builds_manifest_invariant_error() {
        let err = invariant!("broken invariant: {}", "x");
        assert!(matches!(err, Error::ManifestInvariant(_)));
        assert_eq!(err.to_string(), "internal state error: broken invariant: x");
    }

    #[test]
    fn exit_code_is_2_for_cli_input_errors() {
        let invalid_tool = Error::InvalidToolSpec {
            spec: "npm:".to_string(),
            reason: "bad".to_string(),
        };
        let invalid_use = Error::InvalidRuntimeSelector {
            input: "node".to_string(),
        };
        let dup = Error::DuplicateRuntimeUse {
            runtime: "node".to_string(),
        };

        assert_eq!(invalid_tool.exit_code(), 2);
        assert_eq!(invalid_use.exit_code(), 2);
        assert_eq!(dup.exit_code(), 2);
    }

    #[test]
    fn exit_code_is_3_for_user_actionable_state_errors() {
        let unmapped = Error::UnmappedBackendWithoutUse {
            backend: "aqua".to_string(),
        };
        let missing = Error::MissingGlobalRuntime {
            runtime: "node".to_string(),
        };
        let not_installed = Error::RuntimeNotInstalled {
            runtime: "node".to_string(),
        };
        let conflict = Error::CommandOwnershipConflict {
            command: "prettier".to_string(),
            owner: "npm:prettier".to_string(),
            requested: "npm:other".to_string(),
        };
        let tool_missing = Error::ToolNotInstalled {
            tool_id: "npm:prettier".to_string(),
        };
        let orphan = Error::ToolNotInstalledOrphanFound {
            tool_id: "npm:prettier".to_string(),
            path: "/tmp/x".to_string(),
        };

        assert_eq!(unmapped.exit_code(), 3);
        assert_eq!(missing.exit_code(), 3);
        assert_eq!(not_installed.exit_code(), 3);
        assert_eq!(conflict.exit_code(), 3);
        assert_eq!(tool_missing.exit_code(), 3);
        assert_eq!(orphan.exit_code(), 3);
    }

    #[test]
    fn exit_code_is_1_for_internal_errors() {
        let invariant = Error::ManifestInvariant("oops".to_string());
        let mise_fail = Error::MiseCommandFailed {
            command: "mise x".to_string(),
            stderr: "bad".to_string(),
        };
        let io = Error::Io(std::io::Error::other("io"));
        let toml_de: Error = toml::from_str::<toml::Table>("=").unwrap_err().into();
        let toml_ser: Error = toml::to_string(&f64::NAN).unwrap_err().into();
        let json: Error = serde_json::from_str::<serde_json::Value>("{")
            .unwrap_err()
            .into();

        assert_eq!(invariant.exit_code(), 1);
        assert_eq!(mise_fail.exit_code(), 1);
        assert_eq!(io.exit_code(), 1);
        assert_eq!(toml_de.exit_code(), 1);
        assert_eq!(toml_ser.exit_code(), 1);
        assert_eq!(json.exit_code(), 1);
    }
}
