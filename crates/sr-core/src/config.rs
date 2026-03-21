use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::commit::{CommitType, DEFAULT_COMMIT_PATTERN, default_commit_types};
use crate::error::ReleaseError;
use crate::version::BumpLevel;
use crate::version_files::detect_version_files;

/// Preferred config file name for new projects.
pub const DEFAULT_CONFIG_FILE: &str = "sr.yaml";

/// Legacy config file name (deprecated, will be removed in a future release).
pub const LEGACY_CONFIG_FILE: &str = ".urmzd.sr.yml";

/// Config file candidates, checked in priority order.
pub const CONFIG_CANDIDATES: &[&str] = &["sr.yaml", "sr.yml", LEGACY_CONFIG_FILE];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReleaseConfig {
    pub branches: Vec<String>,
    pub tag_prefix: String,
    pub commit_pattern: String,
    pub breaking_section: String,
    pub misc_section: String,
    pub types: Vec<CommitType>,
    pub changelog: ChangelogConfig,
    pub version_files: Vec<String>,
    pub version_files_strict: bool,
    pub artifacts: Vec<String>,
    pub floating_tags: bool,
    pub build_command: Option<String>,
    /// Additional files/globs to stage after `build_command` runs (e.g. `Cargo.lock`).
    pub stage_files: Vec<String>,
    /// Pre-release identifier (e.g. "alpha", "beta", "rc"). When set, versions are
    /// formatted as X.Y.Z-<id>.N where N auto-increments.
    pub prerelease: Option<String>,
    /// Shell command to run before the release starts (validation, checks).
    pub pre_release_command: Option<String>,
    /// Shell command to run after the release completes (notifications, deployments).
    pub post_release_command: Option<String>,
    /// Sign annotated tags with GPG/SSH (git tag -s).
    pub sign_tags: bool,
    /// Create GitHub releases as drafts (requires manual publishing).
    pub draft: bool,
    /// Minijinja template for the GitHub release name.
    /// Available variables: `version`, `tag_name`, `tag_prefix`.
    /// Default when None: uses the tag name (e.g. "v1.2.0").
    pub release_name_template: Option<String>,
    /// Git hooks configuration.
    pub hooks: HooksConfig,
    /// Monorepo packages. When non-empty, each package is released independently.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageConfig>,
    /// Internal: set when resolving a package config. Commits are filtered to this path.
    #[serde(skip)]
    pub path_filter: Option<String>,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            branches: vec!["main".into(), "master".into()],
            tag_prefix: "v".into(),
            commit_pattern: DEFAULT_COMMIT_PATTERN.into(),
            breaking_section: "Breaking Changes".into(),
            misc_section: "Miscellaneous".into(),
            types: default_commit_types(),
            changelog: ChangelogConfig::default(),
            version_files: vec![],
            version_files_strict: false,
            artifacts: vec![],
            floating_tags: false,
            build_command: None,
            stage_files: vec![],
            prerelease: None,
            pre_release_command: None,
            post_release_command: None,
            sign_tags: false,
            draft: false,
            release_name_template: None,
            hooks: HooksConfig::with_defaults(),
            packages: vec![],
            path_filter: None,
        }
    }
}

/// A package in a monorepo. Each package is released independently with its own
/// version, tags, and changelog. Commits are filtered by `path`.
///
/// ```yaml
/// packages:
///   - name: core
///     path: crates/core
///     version_files:
///       - crates/core/Cargo.toml
///   - name: cli
///     path: crates/cli
///     version_files:
///       - crates/cli/Cargo.toml
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    /// Package name — used in the default tag prefix (`{name}/v`).
    pub name: String,
    /// Directory path relative to the repo root. Only commits touching this path trigger a release.
    pub path: String,
    /// Tag prefix override (default: `{name}/v`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_prefix: Option<String>,
    /// Version files override.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_files: Vec<String>,
    /// Changelog override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<ChangelogConfig>,
    /// Build command override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    /// Stage files override.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_files: Vec<String>,
}

/// Git hooks configuration.
///
/// Each key is a git hook name (e.g. `commit-msg`, `pre-commit`, `pre-push`)
/// and the value is a list of shell commands to run sequentially.
/// Hook scripts in `.githooks/` are generated by `sr init`.
///
/// ```yaml
/// hooks:
///   commit-msg:
///     - sr hook commit-msg "$1"
///   pre-commit:
///     - cargo fmt -- --check
///     - cargo clippy --workspace -- -D warnings
///   pre-push:
///     - cargo test --workspace
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HooksConfig {
    pub hooks: BTreeMap<String, Vec<String>>,
}

impl HooksConfig {
    pub fn with_defaults() -> Self {
        let mut hooks = BTreeMap::new();
        hooks.insert("commit-msg".into(), vec!["sr hook commit-msg".into()]);
        Self { hooks }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangelogConfig {
    pub file: Option<String>,
    pub template: Option<String>,
}

impl ReleaseConfig {
    /// Find the first config file that exists in the given directory.
    /// Returns `(path, is_legacy)`.
    pub fn find_config(dir: &Path) -> Option<(std::path::PathBuf, bool)> {
        for &candidate in CONFIG_CANDIDATES {
            let path = dir.join(candidate);
            if path.exists() {
                let is_legacy = candidate == LEGACY_CONFIG_FILE;
                return Some((path, is_legacy));
            }
        }
        None
    }

    /// Load config from a YAML file. Falls back to defaults if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, ReleaseError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents =
            std::fs::read_to_string(path).map_err(|e| ReleaseError::Config(e.to_string()))?;

        serde_yaml_ng::from_str(&contents).map_err(|e| ReleaseError::Config(e.to_string()))
    }

    /// Resolve a package into a full release config by merging package overrides with root config.
    pub fn resolve_package(&self, pkg: &PackageConfig) -> Self {
        let mut config = self.clone();
        config.tag_prefix = pkg
            .tag_prefix
            .clone()
            .unwrap_or_else(|| format!("{}/v", pkg.name));
        config.path_filter = Some(pkg.path.clone());
        if !pkg.version_files.is_empty() {
            config.version_files = pkg.version_files.clone();
        } else if config.version_files.is_empty() {
            // Auto-detect version files in the package directory
            let detected = detect_version_files(Path::new(&pkg.path));
            if !detected.is_empty() {
                config.version_files = detected
                    .into_iter()
                    .map(|f| format!("{}/{f}", pkg.path))
                    .collect();
            }
        }
        if let Some(ref cl) = pkg.changelog {
            config.changelog = cl.clone();
        }
        if let Some(ref cmd) = pkg.build_command {
            config.build_command = Some(cmd.clone());
        }
        if !pkg.stage_files.is_empty() {
            config.stage_files = pkg.stage_files.clone();
        }
        // Clear packages to avoid recursion
        config.packages = vec![];
        config
    }

    /// Find a package by name. Returns an error if the package is not found.
    pub fn find_package(&self, name: &str) -> Result<&PackageConfig, ReleaseError> {
        self.packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| {
                let available: Vec<&str> = self.packages.iter().map(|p| p.name.as_str()).collect();
                ReleaseError::Config(format!(
                    "package '{name}' not found. Available: {}",
                    if available.is_empty() {
                        "(none — no packages configured)".to_string()
                    } else {
                        available.join(", ")
                    }
                ))
            })
    }
}

// Custom deserialization for BumpLevel so it can appear in YAML config.
impl<'de> Deserialize<'de> for BumpLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "major" => Ok(BumpLevel::Major),
            "minor" => Ok(BumpLevel::Minor),
            "patch" => Ok(BumpLevel::Patch),
            _ => Err(serde::de::Error::custom(format!("unknown bump level: {s}"))),
        }
    }
}

impl Serialize for BumpLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            BumpLevel::Major => "major",
            BumpLevel::Minor => "minor",
            BumpLevel::Patch => "patch",
        };
        serializer.serialize_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_values() {
        let config = ReleaseConfig::default();
        assert_eq!(config.branches, vec!["main", "master"]);
        assert_eq!(config.tag_prefix, "v");
        assert_eq!(config.commit_pattern, DEFAULT_COMMIT_PATTERN);
        assert_eq!(config.breaking_section, "Breaking Changes");
        assert_eq!(config.misc_section, "Miscellaneous");
        assert!(!config.types.is_empty());
        assert!(!config.version_files_strict);
        assert!(config.artifacts.is_empty());
        assert!(!config.floating_tags);
    }

    #[test]
    fn load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.yml");
        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.tag_prefix, "v");
    }

    #[test]
    fn load_valid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "branches:\n  - develop\ntag_prefix: release-").unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.branches, vec!["develop"]);
        assert_eq!(config.tag_prefix, "release-");
    }

    #[test]
    fn load_partial_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(&path, "tag_prefix: rel-\n").unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.tag_prefix, "rel-");
        assert_eq!(config.branches, vec!["main", "master"]);
        // defaults should still apply for types/pattern/breaking_section
        assert_eq!(config.commit_pattern, DEFAULT_COMMIT_PATTERN);
        assert_eq!(config.breaking_section, "Breaking Changes");
        assert!(!config.types.is_empty());
    }

    #[test]
    fn load_yaml_with_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            "artifacts:\n  - \"dist/*.tar.gz\"\n  - \"build/output-*\"\n",
        )
        .unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.artifacts, vec!["dist/*.tar.gz", "build/output-*"]);
        // defaults still apply
        assert_eq!(config.tag_prefix, "v");
    }

    #[test]
    fn load_yaml_with_floating_tags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(&path, "floating_tags: true\n").unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert!(config.floating_tags);
        // defaults still apply
        assert_eq!(config.tag_prefix, "v");
    }

    #[test]
    fn bump_level_roundtrip() {
        for (level, expected) in [
            (BumpLevel::Major, "major"),
            (BumpLevel::Minor, "minor"),
            (BumpLevel::Patch, "patch"),
        ] {
            let yaml = serde_yaml_ng::to_string(&level).unwrap();
            assert!(yaml.contains(expected));
            let parsed: BumpLevel = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(parsed, level);
        }
    }

    #[test]
    fn types_roundtrip() {
        let config = ReleaseConfig::default();
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        let parsed: ReleaseConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.types.len(), config.types.len());
        assert_eq!(parsed.types[0].name, "feat");
        assert_eq!(parsed.commit_pattern, config.commit_pattern);
        assert_eq!(parsed.breaking_section, config.breaking_section);
    }

    #[test]
    fn load_yaml_with_packages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            r#"
packages:
  - name: core
    path: crates/core
    version_files:
      - crates/core/Cargo.toml
  - name: cli
    path: crates/cli
    tag_prefix: "cli-v"
"#,
        )
        .unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.packages.len(), 2);
        assert_eq!(config.packages[0].name, "core");
        assert_eq!(config.packages[0].path, "crates/core");
        assert_eq!(config.packages[1].tag_prefix.as_deref(), Some("cli-v"));
    }

    #[test]
    fn resolve_package_defaults() {
        let mut config = ReleaseConfig::default();
        config.packages = vec![PackageConfig {
            name: "core".into(),
            path: "crates/core".into(),
            tag_prefix: None,
            version_files: vec![],
            changelog: None,
            build_command: None,
            stage_files: vec![],
        }];

        let resolved = config.resolve_package(&config.packages[0]);
        assert_eq!(resolved.tag_prefix, "core/v");
        assert_eq!(resolved.path_filter.as_deref(), Some("crates/core"));
        // Inherits root config values
        assert_eq!(resolved.branches, config.branches);
        assert!(resolved.packages.is_empty());
    }

    #[test]
    fn resolve_package_overrides() {
        let mut config = ReleaseConfig::default();
        config.version_files = vec!["Cargo.toml".into()];
        config.packages = vec![PackageConfig {
            name: "cli".into(),
            path: "crates/cli".into(),
            tag_prefix: Some("cli-v".into()),
            version_files: vec!["crates/cli/Cargo.toml".into()],
            changelog: Some(ChangelogConfig {
                file: Some("crates/cli/CHANGELOG.md".into()),
                template: None,
            }),
            build_command: Some("cargo build -p cli".into()),
            stage_files: vec!["crates/cli/Cargo.lock".into()],
        }];

        let resolved = config.resolve_package(&config.packages[0]);
        assert_eq!(resolved.tag_prefix, "cli-v");
        assert_eq!(resolved.version_files, vec!["crates/cli/Cargo.toml"]);
        assert_eq!(
            resolved.changelog.file.as_deref(),
            Some("crates/cli/CHANGELOG.md")
        );
        assert_eq!(
            resolved.build_command.as_deref(),
            Some("cargo build -p cli")
        );
        assert_eq!(resolved.stage_files, vec!["crates/cli/Cargo.lock"]);
    }

    #[test]
    fn find_package_found() {
        let mut config = ReleaseConfig::default();
        config.packages = vec![PackageConfig {
            name: "core".into(),
            path: "crates/core".into(),
            tag_prefix: None,
            version_files: vec![],
            changelog: None,
            build_command: None,
            stage_files: vec![],
        }];

        let pkg = config.find_package("core").unwrap();
        assert_eq!(pkg.name, "core");
    }

    #[test]
    fn find_package_not_found() {
        let config = ReleaseConfig::default();
        let err = config.find_package("nonexistent").unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
        assert!(err.to_string().contains("no packages configured"));
    }

    #[test]
    fn packages_not_serialized_when_empty() {
        let config = ReleaseConfig::default();
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        assert!(!yaml.contains("packages"));
    }
}
