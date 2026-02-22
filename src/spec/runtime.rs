use std::{
    collections::BTreeMap,
    convert::Infallible,
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use crate::error::Error;

/// Runtime identifier (`node`, `ruby`, ...).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Runtime {
    Node,
    Ruby,
    Python,
    Rust,
    Go,
    Other(String),
}

impl Runtime {
    fn parse(input: &str) -> Self {
        match input {
            "node" => Self::Node,
            "ruby" => Self::Ruby,
            "python" => Self::Python,
            "rust" => Self::Rust,
            "go" => Self::Go,
            _ => Self::Other(input.to_string()),
        }
    }

    /// `<runtime>@<selector>` (for example `node@lts`)
    pub fn with_selector(&self, selector: &str) -> String {
        format!("{self}@{selector}")
    }

    /// `<runtime>-<version>` for variant keys
    pub fn with_version(&self, version: &str) -> String {
        format!("{self}-{version}")
    }
}

impl AsRef<str> for Runtime {
    fn as_ref(&self) -> &str {
        match self {
            Self::Node => "node",
            Self::Ruby => "ruby",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Other(value) => value,
        }
    }
}

impl From<&str> for Runtime {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl FromStr for Runtime {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(value.into())
    }
}

/// Runtime selector input (`<runtime>@<selector>`) from `--use`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSpec {
    runtime: Runtime,
    selector: String,
}

impl RuntimeSpec {
    pub fn new(runtime: Runtime, selector: impl Into<String>) -> Self {
        let selector = selector.into();

        debug_assert!(
            !selector.is_empty(),
            "RuntimeSpec::new requires non-empty selector"
        );

        Self { runtime, selector }
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    /// `<runtime>@<selector>`
    pub fn runtime_pin(&self) -> String {
        self.runtime.with_selector(self.selector())
    }
}

impl Display for RuntimeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.runtime_pin())
    }
}

impl FromStr for RuntimeSpec {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (runtime, selector) =
            input
                .split_once('@')
                .ok_or_else(|| Error::InvalidRuntimeSelector {
                    input: input.to_string(),
                })?;

        if runtime.is_empty() || selector.is_empty() {
            return Err(Error::InvalidRuntimeSelector {
                input: input.to_string(),
            });
        }

        if runtime.bytes().any(|b| b.is_ascii_uppercase()) {
            return Err(Error::InvalidRuntimeSelector {
                input: input.to_string(),
            });
        }

        Ok(Self::new(runtime.into(), selector.to_string()))
    }
}

/// Runtime specs keyed by runtime.
///
/// Used for both CLI `--use` selectors and resolved concrete pins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimePins(BTreeMap<Runtime, RuntimeSpec>);

impl RuntimePins {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

impl TryFrom<Vec<RuntimeSpec>> for RuntimePins {
    type Error = Error;

    fn try_from(uses: Vec<RuntimeSpec>) -> Result<Self, Self::Error> {
        let mut pins = Self::new();

        for spec in uses {
            let runtime = spec.runtime().clone();
            if pins.insert(runtime.clone(), spec).is_some() {
                return Err(Error::DuplicateRuntimeUse {
                    runtime: runtime.to_string(),
                });
            }
        }

        Ok(pins)
    }
}

impl Deref for RuntimePins {
    type Target = BTreeMap<Runtime, RuntimeSpec>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RuntimePins {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Runtime, RuntimePins, RuntimeSpec};
    use crate::error::Error;

    #[test]
    fn runtime_spec_from_str_validates_shape() {
        assert!("node@22".parse::<RuntimeSpec>().is_ok());
        assert!("node".parse::<RuntimeSpec>().is_err());
        assert!("@22".parse::<RuntimeSpec>().is_err());
        assert!("node@".parse::<RuntimeSpec>().is_err());
        assert!("Node@22".parse::<RuntimeSpec>().is_err());
    }

    #[test]
    fn runtime_helpers_format_consistently() {
        let runtime = Runtime::Node;
        assert_eq!(runtime.with_selector("lts"), "node@lts");
        assert_eq!(runtime.with_version("24.13.1"), "node-24.13.1");
        assert_eq!(Runtime::Ruby.with_selector("3.3"), "ruby@3.3");
    }

    #[test]
    fn runtime_display_and_parse_are_stable() {
        assert_eq!(Runtime::Node.to_string(), "node");
        assert_eq!(Runtime::Other("vfox".to_string()).to_string(), "vfox");
        assert_eq!("node".parse::<Runtime>().unwrap(), Runtime::Node);
        assert_eq!(
            "VFOX".parse::<Runtime>().unwrap(),
            Runtime::Other("VFOX".to_string())
        );
    }

    #[test]
    fn runtime_spec_display_is_stable() {
        let spec: RuntimeSpec = "python@3.12".parse().unwrap();
        assert_eq!(spec.to_string(), "python@3.12");
        assert_eq!(spec.runtime_pin(), "python@3.12");
    }

    #[test]
    fn runtime_pins_try_from_accepts_distinct_runtimes() {
        let pins = RuntimePins::try_from(vec![
            RuntimeSpec::new(Runtime::Node, "22"),
            RuntimeSpec::new(Runtime::Python, "3.12"),
        ])
        .unwrap();

        assert_eq!(
            pins.get(&Runtime::Node).unwrap(),
            &RuntimeSpec::new(Runtime::Node, "22")
        );
        assert_eq!(
            pins.get(&Runtime::Python).unwrap(),
            &RuntimeSpec::new(Runtime::Python, "3.12")
        );
    }

    #[test]
    fn runtime_pins_try_from_rejects_duplicate_runtime() {
        let err = RuntimePins::try_from(vec![
            RuntimeSpec::new(Runtime::Node, "22"),
            RuntimeSpec::new(Runtime::Node, "lts"),
        ])
        .unwrap_err();

        assert!(matches!(err, Error::DuplicateRuntimeUse { runtime } if runtime == "node"));
    }
}
