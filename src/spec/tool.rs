use std::{fmt::Display, str::FromStr};

use crate::error::Error;
use crate::fs::{Path, PathBuf};

use super::Backend;

/// Canonical tool identity (`<backend>:<name>`), always unversioned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolId {
    backend: Backend,
    name: String,
}

impl ToolId {
    fn new(backend: Backend, name: String) -> Self {
        debug_assert!(!name.is_empty(), "ToolId::new requires non-empty name",);

        Self { backend, name }
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn key(&self) -> String {
        format!(
            "{}-{}",
            to_kebab_case(self.backend.as_ref()),
            to_kebab_case(&self.name)
        )
    }

    pub fn layout(&self, root: &Path) -> ToolLayout {
        ToolLayout::new(root, self.key())
    }
}

impl Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.backend, self.name)
    }
}

impl FromStr for ToolId {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.parse()? {
            ToolSpec {
                tool_id,
                version: None,
            } => Ok(tool_id),
            ToolSpec {
                tool_id: _,
                version: Some(_),
            } => Err(Error::InvalidToolSpec {
                spec: input.to_string(),
                reason: "version is not accepted here; expected <backend>:<name>".to_string(),
            }),
        }
    }
}

/// Tool request spec (`<backend>:<name>[@version]`) as accepted by CLI input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    tool_id: ToolId,
    version: Option<String>,
}

impl ToolSpec {
    fn new(tool_id: ToolId, version: Option<String>) -> Self {
        debug_assert!(
            version.as_deref().is_none_or(|v| !v.is_empty()),
            "ToolSpec::new requires version to be None or non-empty"
        );

        Self { tool_id, version }
    }

    pub fn tool_id(&self) -> &ToolId {
        &self.tool_id
    }

    /// fs-safe tool key used under `~/.miseo`.
    pub fn key(&self) -> String {
        self.tool_id.key()
    }

    pub fn backend(&self) -> &Backend {
        self.tool_id.backend()
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Ensure a version is present, defaulting to `latest`.
    pub fn or_latest(&self) -> Self {
        let version = self.version.clone().unwrap_or_else(|| "latest".to_string());
        Self::new(self.tool_id.clone(), Some(version))
    }

    /// Replace the version component with an explicit version.
    pub fn with_version(&self, version: &str) -> Self {
        Self::new(self.tool_id.clone(), Some(version.to_string()))
    }

    pub fn layout(&self, root: &Path) -> ToolLayout {
        ToolLayout::new(root, self.key())
    }
}

impl From<ToolId> for ToolSpec {
    fn from(tool_id: ToolId) -> Self {
        Self::new(tool_id, None)
    }
}

impl Display for ToolSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}@{}", self.tool_id, v),
            None => write!(f, "{}", self.tool_id),
        }
    }
}

impl FromStr for ToolSpec {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let invalid = |reason: &str| Error::InvalidToolSpec {
            spec: input.to_string(),
            reason: reason.to_string(),
        };

        let (backend, remainder) = input
            .split_once(':')
            .ok_or_else(|| invalid("missing backend separator ':'"))?;

        if backend.is_empty() {
            return Err(invalid("backend cannot be empty"));
        }
        if backend.bytes().any(|b| b.is_ascii_uppercase()) {
            return Err(invalid("backend must be lowercase"));
        }

        if remainder.is_empty() {
            return Err(invalid("package name cannot be empty"));
        }

        let (name, version) =
            parse_name_and_version(remainder).map_err(|reason| Error::InvalidToolSpec {
                spec: input.to_string(),
                reason,
            })?;

        let tool_id = ToolId::new(backend.into(), name);

        Ok(Self::new(tool_id, version))
    }
}

fn parse_name_and_version(input: &str) -> Result<(String, Option<String>), String> {
    if let Some(idx) = input.rfind('@') {
        if idx == 0 {
            return Ok((input.to_string(), None));
        }

        if idx == input.len() - 1 {
            return Err("version cannot be empty after '@'".to_string());
        }

        let (name, version_with_at) = input.split_at(idx);
        let version = &version_with_at[1..];

        return Ok((name.to_string(), Some(version.to_string())));
    }

    Ok((input.to_string(), None))
}

fn to_kebab_case(input: &str) -> String {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

/// Tool-level filesystem layout rooted at `~/.miseo/<tool-key>`.
#[derive(Debug, Clone)]
pub struct ToolLayout {
    root: PathBuf,
    tool_key: String,
}

impl ToolLayout {
    fn new(root: &Path, tool_key: impl Into<String>) -> Self {
        Self {
            root: root.to_path_buf(),
            tool_key: tool_key.into(),
        }
    }

    pub fn tool_key(&self) -> &str {
        &self.tool_key
    }

    pub fn tool_dir(&self) -> PathBuf {
        self.root.join(&self.tool_key)
    }

    pub fn root_bin_dir(&self) -> PathBuf {
        self.root.join(".bin")
    }

    /// Build paths for one concrete install variant.
    pub fn variant(&self, variant_key: &str) -> VariantLayout {
        let tool_dir = self.tool_dir();
        let variant_dir = tool_dir.join(variant_key);

        VariantLayout {
            root_bin_dir: self.root_bin_dir(),
            tool_dir: tool_dir.clone(),
            variant_dir: variant_dir.clone(),
            current_link: tool_dir.join("current"),
            local_bin_dir: variant_dir.join(".miseo"),
        }
    }
}

/// Variant-level filesystem layout for one concrete install.
#[derive(Debug, Clone)]
pub struct VariantLayout {
    root_bin_dir: PathBuf,
    tool_dir: PathBuf,
    variant_dir: PathBuf,
    current_link: PathBuf,
    local_bin_dir: PathBuf,
}

impl VariantLayout {
    pub fn root_bin_dir(&self) -> &Path {
        &self.root_bin_dir
    }

    pub fn tool_dir(&self) -> &Path {
        &self.tool_dir
    }

    pub fn variant_dir(&self) -> &Path {
        &self.variant_dir
    }

    pub fn current_link(&self) -> &Path {
        &self.current_link
    }

    pub fn local_bin_dir(&self) -> &Path {
        &self.local_bin_dir
    }

    pub fn local_command(&self, command: &str) -> PathBuf {
        self.local_bin_dir.join(command)
    }

    pub fn public_command(&self, command: &str) -> PathBuf {
        self.root_bin_dir.join(command)
    }

    pub fn public_target(&self, command: &str) -> PathBuf {
        self.tool_dir.join("current/.miseo").join(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_and_versioned_specs() {
        let s: ToolSpec = "npm:http-server".parse().unwrap();
        assert_eq!(s.backend(), &Backend::Npm);
        assert_eq!(s.tool_id().name(), "http-server");
        assert_eq!(s.to_string(), "npm:http-server");
        assert_eq!(s.tool_id().to_string(), "npm:http-server");

        let s: ToolSpec = "npm:http-server@14".parse().unwrap();
        assert_eq!(s.to_string(), "npm:http-server@14");
        assert_eq!(s.tool_id().to_string(), "npm:http-server");
    }

    #[test]
    fn parses_scoped_names() {
        let s: ToolSpec = "npm:@antfu/ni".parse().unwrap();
        assert_eq!(s.tool_id().name(), "@antfu/ni");
        assert_eq!(s.to_string(), "npm:@antfu/ni");

        let s: ToolSpec = "npm:@antfu/ni@1.0.0".parse().unwrap();
        assert_eq!(s.tool_id().name(), "@antfu/ni");
        assert_eq!(s.to_string(), "npm:@antfu/ni@1.0.0");
    }

    #[test]
    fn rejects_invalid_specs() {
        assert!("npm".parse::<ToolSpec>().is_err());
        assert!(":foo".parse::<ToolSpec>().is_err());
        assert!("npm:".parse::<ToolSpec>().is_err());
        assert!("npm:foo@".parse::<ToolSpec>().is_err());
        assert!("NPM:foo".parse::<ToolSpec>().is_err());
    }

    #[test]
    fn tool_key_normalization_matches_examples() {
        assert_eq!(to_kebab_case("npm:http-server"), "npm-http-server");
        assert_eq!(to_kebab_case("npm:@antfu/ni"), "npm-antfu-ni");
        assert_eq!(
            to_kebab_case("vfox:version-fox/vfox-nodejs"),
            "vfox-version-fox-vfox-nodejs"
        );
    }

    #[test]
    fn tool_spec_display_round_trips_shape() {
        let s: ToolSpec = "npm:prettier@3".parse().unwrap();
        assert_eq!(s.to_string(), "npm:prettier@3");
        let s: ToolSpec = "npm:prettier".parse().unwrap();
        assert_eq!(s.to_string(), "npm:prettier");
    }

    #[test]
    fn tool_id_rejects_versioned_input() {
        assert!("npm:prettier".parse::<ToolId>().is_ok());
        assert!("npm:prettier@3".parse::<ToolId>().is_err());
    }

    #[test]
    fn tool_id_rejects_versioned_input_from_str() {
        let err = "npm:prettier@3".parse::<ToolId>().unwrap_err();
        assert!(matches!(err, Error::InvalidToolSpec { .. }));
    }

    #[test]
    fn tool_layout_builds_expected_paths() {
        let root = Path::new("/tmp/miseo");
        let tool_id: ToolId = "npm:prettier".parse().unwrap();
        let layout = tool_id.layout(root);

        assert_eq!(layout.tool_key(), "npm-prettier");
        assert_eq!(layout.root_bin_dir(), PathBuf::from("/tmp/miseo/.bin"));
        assert_eq!(layout.tool_dir(), PathBuf::from("/tmp/miseo/npm-prettier"));
    }

    #[test]
    fn variant_layout_builds_command_paths() {
        let root = Path::new("/tmp/miseo");
        let tool_id: ToolId = "npm:prettier".parse().unwrap();
        let variant = tool_id.layout(root).variant("3.8.1+node-24.13.1");

        assert_eq!(
            variant.local_command("prettier"),
            PathBuf::from("/tmp/miseo/npm-prettier/3.8.1+node-24.13.1/.miseo/prettier")
        );
        assert_eq!(
            variant.public_target("prettier"),
            PathBuf::from("/tmp/miseo/npm-prettier/current/.miseo/prettier")
        );
        assert_eq!(
            variant.public_command("prettier"),
            PathBuf::from("/tmp/miseo/.bin/prettier")
        );
    }
}
