use std::{convert::Infallible, fmt::Display, str::FromStr};

use super::Runtime;

/// Package backend identifier (`npm`, `gem`, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backend {
    Npm,
    Gem,
    Pipx,
    Cargo,
    Go,
    Other(String),
}

impl Backend {
    /// Default runtime families used when `--use` is omitted.
    pub fn default_runtimes(&self) -> Option<&'static [Runtime]> {
        match self {
            Self::Npm => Some(&[Runtime::Node]),
            Self::Gem => Some(&[Runtime::Ruby]),
            Self::Pipx => Some(&[Runtime::Python]),
            Self::Cargo => Some(&[Runtime::Rust]),
            Self::Go => Some(&[Runtime::Go]),
            Self::Other(_) => None,
        }
    }

    fn parse(input: &str) -> Self {
        match input {
            "npm" => Self::Npm,
            "gem" => Self::Gem,
            "pipx" => Self::Pipx,
            "cargo" => Self::Cargo,
            "go" => Self::Go,
            _ => Self::Other(input.to_string()),
        }
    }
}

impl AsRef<str> for Backend {
    fn as_ref(&self) -> &str {
        match self {
            Self::Npm => "npm",
            Self::Gem => "gem",
            Self::Pipx => "pipx",
            Self::Cargo => "cargo",
            Self::Go => "go",
            Self::Other(value) => value,
        }
    }
}

impl From<&str> for Backend {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl FromStr for Backend {
    type Err = Infallible;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(input.into())
    }
}

#[cfg(test)]
mod tests {
    use super::Backend;
    use crate::spec::Runtime;

    #[test]
    fn default_runtime_mapping() {
        assert_eq!(Backend::Npm.default_runtimes().unwrap(), &[Runtime::Node]);
        assert_eq!(Backend::Gem.default_runtimes().unwrap(), &[Runtime::Ruby]);
        assert_eq!(
            Backend::Pipx.default_runtimes().unwrap(),
            &[Runtime::Python]
        );
        assert_eq!(Backend::Cargo.default_runtimes().unwrap(), &[Runtime::Rust]);
        assert_eq!(Backend::Go.default_runtimes().unwrap(), &[Runtime::Go]);
        assert!(
            Backend::Other("aqua".to_string())
                .default_runtimes()
                .is_none()
        );
    }

    #[test]
    fn backend_parse_is_case_sensitive_and_preserves_other_input() {
        assert_eq!("npm".parse::<Backend>().unwrap(), Backend::Npm);
        assert_eq!(
            "NPM".parse::<Backend>().unwrap(),
            Backend::Other("NPM".to_string())
        );

        let other: Backend = "VFOX".parse().unwrap();
        assert_eq!(other, Backend::Other("VFOX".to_string()));
        assert_eq!(other.to_string(), "VFOX");
    }
}
