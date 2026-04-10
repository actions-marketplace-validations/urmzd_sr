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
    /// Structured lifecycle hooks for the release pipeline.
    /// Each step runs at a specific point in the release flow.
    /// For simple use cases, `pre_release_command` and `post_release_command` are still supported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<LifecycleStep>,
    /// Versioning strategy for monorepo packages.
    /// `independent` (default): each package is versioned separately.
    /// `fixed`: all packages share one version, tag, and changelog.
    #[serde(default)]
    pub versioning: VersioningMode,
    /// Monorepo packages. When non-empty, each package is released independently
    /// (or together when `versioning: fixed`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageConfig>,
    /// Internal: set when resolving a package config. Commits are filtered to this path.
    #[serde(skip)]
    pub path_filter: Option<String>,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            branches: vec!["main".into()],
            tag_prefix: "v".into(),
            commit_pattern: DEFAULT_COMMIT_PATTERN.into(),
            breaking_section: "Breaking Changes".into(),
            misc_section: "Miscellaneous".into(),
            types: default_commit_types(),
            changelog: ChangelogConfig::default(),
            version_files: vec![],
            version_files_strict: false,
            artifacts: vec![],
            floating_tags: true,
            build_command: None,
            stage_files: vec![],
            prerelease: None,
            pre_release_command: None,
            post_release_command: None,
            sign_tags: false,
            draft: false,
            release_name_template: None,
            hooks: HooksConfig::with_defaults(),
            lifecycle: vec![],
            versioning: VersioningMode::default(),
            packages: vec![],
            path_filter: None,
        }
    }
}

/// Versioning strategy for monorepo packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VersioningMode {
    /// Each package is versioned and released independently (default).
    #[default]
    Independent,
    /// All packages share a single version, tag, and changelog.
    Fixed,
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

/// A lifecycle event in the release pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    /// Before the release starts (validation, checks, linting).
    PreRelease,
    /// After version files are bumped but before build_command runs.
    PostBump,
    /// After build_command completes but before git commit.
    PostBuild,
    /// After everything completes (notifications, deployments).
    PostRelease,
}

/// A named step in the release lifecycle pipeline.
///
/// ```yaml
/// lifecycle:
///   - name: lint
///     when: pre_release
///     run: "cargo clippy -- -D warnings"
///   - name: notify
///     when: post_release
///     run: "./scripts/notify-slack.sh"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LifecycleStep {
    /// Step name (for logging).
    pub name: String,
    /// When this step runs in the release pipeline.
    pub when: LifecycleEvent,
    /// Shell command to execute. `SR_VERSION` and `SR_TAG` env vars are set.
    pub run: String,
}

/// A single entry in a hook's command list.
///
/// Can be either a simple shell command string or a structured step with
/// file-pattern matching.
///
/// ```yaml
/// hooks:
///   commit-msg:
///     - sr hook commit-msg          # simple command
///   pre-commit:
///     - step: format                # structured step
///       patterns:
///         - "*.rs"
///       rules:
///         - "rustfmt --check --edition 2024 {files}"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookEntry {
    Step {
        step: String,
        patterns: Vec<String>,
        rules: Vec<String>,
    },
    Simple(String),
}

/// Git hooks configuration.
///
/// Each key is a git hook name (e.g. `commit-msg`, `pre-commit`, `pre-push`)
/// and the value is a list of entries — either simple shell commands or
/// structured steps with file-pattern matching.
///
/// Hook scripts in `.githooks/` are generated by `sr init`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HooksConfig {
    pub hooks: BTreeMap<String, Vec<HookEntry>>,
}

impl HooksConfig {
    pub fn with_defaults() -> Self {
        let mut hooks = BTreeMap::new();
        hooks.insert(
            "commit-msg".into(),
            vec![HookEntry::Simple("sr hook commit-msg".into())],
        );
        Self { hooks }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangelogConfig {
    pub file: Option<String>,
    pub template: Option<String>,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        Self {
            file: Some("CHANGELOG.md".into()),
            template: None,
        }
    }
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

    /// Resolve all packages into a single config for fixed versioning mode.
    ///
    /// Collects version files, stage files, and build commands from all packages.
    /// Uses the root `tag_prefix` (no per-package prefixes). No path filter is set
    /// so commits from the entire repo determine the version bump.
    pub fn resolve_fixed(&self) -> Self {
        let mut config = self.clone();
        // No path filter — all commits count
        config.path_filter = None;

        // Collect version files from all packages
        let mut version_files: Vec<String> = config.version_files.clone();
        for pkg in &self.packages {
            if !pkg.version_files.is_empty() {
                version_files.extend(pkg.version_files.clone());
            } else {
                let detected = detect_version_files(Path::new(&pkg.path));
                version_files.extend(
                    detected
                        .into_iter()
                        .map(|f| format!("{}/{f}", pkg.path)),
                );
            }
        }
        version_files.sort();
        version_files.dedup();
        config.version_files = version_files;

        // Collect stage files from all packages
        let mut stage_files = config.stage_files.clone();
        for pkg in &self.packages {
            stage_files.extend(pkg.stage_files.clone());
        }
        stage_files.sort();
        stage_files.dedup();
        config.stage_files = stage_files;

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

/// Generate a fully-commented default `sr.yaml` template.
///
/// The returned string is valid YAML with inline comments documenting every field.
/// `version_files` is injected dynamically (typically from auto-detection).
pub fn default_config_template(version_files: &[String]) -> String {
    let vf = if version_files.is_empty() {
        "version_files: []\n".to_string()
    } else {
        let mut s = "version_files:\n".to_string();
        for f in version_files {
            s.push_str(&format!("  - {f}\n"));
        }
        s
    };

    format!(
        r#"# sr configuration
# Full reference: https://github.com/urmzd/sr#configuration

# Branches that trigger releases when commits are pushed.
branches:
  - main

# Prefix prepended to version tags (e.g. "v1.2.0").
tag_prefix: "v"

# Regex for parsing conventional commits.
# Required named groups: type, description.
# Optional named groups: scope, breaking.
commit_pattern: '^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s+(?P<description>.+)'

# Changelog section heading for breaking changes.
breaking_section: Breaking Changes

# Fallback changelog section for unrecognised commit types.
misc_section: Miscellaneous

# Commit type definitions.
# name:    commit type prefix (e.g. "feat", "fix")
# bump:    version bump level — major, minor, patch, or omit for no bump
# section: changelog section heading, or omit to exclude from changelog
# pattern: optional regex to match non-conventional commits as this type (fallback)
types:
  - name: feat
    bump: minor
    section: Features
  - name: fix
    bump: patch
    section: Bug Fixes
  - name: perf
    bump: patch
    section: Performance
  - name: docs
    section: Documentation
  - name: refactor
    bump: patch
    section: Refactoring
  - name: revert
    section: Reverts
  - name: chore
  - name: ci
  - name: test
  - name: build
  - name: style

# Changelog configuration.
# file:     path to the changelog file (e.g. CHANGELOG.md), or omit to skip writing
# template: custom Minijinja template string for changelog rendering
changelog:
  file: CHANGELOG.md
  template:

# Manifest files to bump on release (e.g. Cargo.toml, package.json, pyproject.toml).
# Auto-detected if empty.
{vf}
# Fail if a version file uses an unsupported format (default: skip unknown files).
version_files_strict: false

# Glob patterns for release assets to upload to GitHub (e.g. "dist/*.tar.gz").
artifacts: []

# Create floating major version tags (e.g. "v3" pointing to latest v3.x.x).
floating_tags: true

# Shell command to run after version files are bumped (e.g. "cargo build --release").
build_command:

# Additional files/globs to stage after build_command runs (e.g. Cargo.lock).
stage_files: []

# Pre-release identifier (e.g. "alpha", "beta", "rc").
# When set, versions are formatted as X.Y.Z-<id>.N where N auto-increments.
prerelease:

# Shell command to run before the release starts (validation, checks).
pre_release_command:

# Shell command to run after the release completes (notifications, deployments).
post_release_command:

# Sign annotated tags with GPG/SSH (git tag -s).
sign_tags: false

# Create GitHub releases as drafts (requires manual publishing).
draft: false

# Minijinja template for the GitHub release name.
# Available variables: version, tag_name, tag_prefix.
# Default: uses the tag name (e.g. "v1.2.0").
release_name_template:

# Git hooks configuration.
# Each key is a git hook name. Values can be simple commands or structured steps.
# Steps with patterns only run when staged files match the globs.
# Rules containing {{files}} receive the matched file list.
# Hook scripts are generated in .githooks/ by "sr init".
hooks:
  commit-msg:
    - sr hook commit-msg
  # pre-commit:
  #   - step: format
  #     patterns:
  #       - "*.rs"
  #     rules:
  #       - "rustfmt --check --edition 2024 {{files}}"
  #   - step: lint
  #     patterns:
  #       - "*.rs"
  #     rules:
  #       - "cargo clippy --workspace -- -D warnings"

# Release lifecycle hooks — run commands at specific points in the release pipeline.
# Runs after pre_release_command/post_release_command (both systems coexist).
# Supported events: pre_release, post_bump, post_build, post_release
# SR_VERSION and SR_TAG env vars are set for all lifecycle steps.
# lifecycle:
#   - name: lint
#     when: pre_release
#     run: "cargo clippy -- -D warnings"
#   - name: verify
#     when: post_bump
#     run: "./scripts/verify-version.sh"
#   - name: check
#     when: post_build
#     run: "./scripts/check-artifacts.sh"
#   - name: notify
#     when: post_release
#     run: "./scripts/notify-slack.sh"

# Versioning strategy for monorepo packages.
# "independent" (default): each package gets its own version and tags.
# "fixed": all packages share one version, tag, and changelog.
# versioning: independent

# Monorepo packages (uncomment and configure if needed).
# With versioning: independent — each package is released separately.
# With versioning: fixed — all packages are released together under one version.
# packages:
#   - name: core
#     path: crates/core
#     tag_prefix: "core/v"          # default: "<name>/v" (independent only)
#     version_files:
#       - crates/core/Cargo.toml
#     changelog:
#       file: crates/core/CHANGELOG.md
#     build_command: cargo build -p core
#     stage_files:
#       - crates/core/Cargo.lock
"#
    )
}

/// Merge new default fields into an existing config YAML string.
///
/// Adds any top-level or nested mapping keys present in the defaults but missing
/// from the existing config. Arrays are never merged (user's array = user's intent).
/// Returns the merged YAML with a header comment.
pub fn merge_config_yaml(existing_yaml: &str) -> Result<String, ReleaseError> {
    let mut existing: serde_yaml_ng::Value = serde_yaml_ng::from_str(existing_yaml)
        .map_err(|e| ReleaseError::Config(format!("failed to parse existing config: {e}")))?;

    let default_config = ReleaseConfig::default();
    let default_yaml = serde_yaml_ng::to_string(&default_config)
        .map_err(|e| ReleaseError::Config(e.to_string()))?;
    let defaults: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&default_yaml).map_err(|e| ReleaseError::Config(e.to_string()))?;

    deep_merge_value(&mut existing, &defaults);

    let merged =
        serde_yaml_ng::to_string(&existing).map_err(|e| ReleaseError::Config(e.to_string()))?;

    Ok(format!(
        "# sr configuration — merged with new defaults\n\
         # Run 'sr init --force' for a fully-commented template.\n\n\
         {merged}"
    ))
}

/// Recursively insert missing keys from `defaults` into `base`.
/// Only mapping keys are merged; arrays and scalars are left untouched.
fn deep_merge_value(base: &mut serde_yaml_ng::Value, defaults: &serde_yaml_ng::Value) {
    use serde_yaml_ng::Value;
    if let (Value::Mapping(base_map), Value::Mapping(default_map)) = (base, defaults) {
        for (key, default_val) in default_map {
            match base_map.get_mut(key) {
                Some(existing_val) => {
                    // Recurse into nested mappings only
                    if matches!(default_val, Value::Mapping(_)) {
                        deep_merge_value(existing_val, default_val);
                    }
                }
                None => {
                    base_map.insert(key.clone(), default_val.clone());
                }
            }
        }
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
        assert_eq!(config.branches, vec!["main"]);
        assert_eq!(config.tag_prefix, "v");
        assert_eq!(config.commit_pattern, DEFAULT_COMMIT_PATTERN);
        assert_eq!(config.breaking_section, "Breaking Changes");
        assert_eq!(config.misc_section, "Miscellaneous");
        assert!(!config.types.is_empty());
        assert!(!config.version_files_strict);
        assert!(config.artifacts.is_empty());
        assert!(config.floating_tags);
        assert_eq!(config.changelog.file.as_deref(), Some("CHANGELOG.md"));
        // Verify refactor has bump: patch (must match README)
        let refactor = config.types.iter().find(|t| t.name == "refactor").unwrap();
        assert_eq!(refactor.bump, Some(BumpLevel::Patch));
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
        assert_eq!(config.branches, vec!["main"]);
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
        let config = ReleaseConfig {
            packages: vec![PackageConfig {
                name: "core".into(),
                path: "crates/core".into(),
                tag_prefix: None,
                version_files: vec![],
                changelog: None,
                build_command: None,
                stage_files: vec![],
            }],
            ..Default::default()
        };

        let resolved = config.resolve_package(&config.packages[0]);
        assert_eq!(resolved.tag_prefix, "core/v");
        assert_eq!(resolved.path_filter.as_deref(), Some("crates/core"));
        // Inherits root config values
        assert_eq!(resolved.branches, config.branches);
        assert!(resolved.packages.is_empty());
    }

    #[test]
    fn resolve_package_overrides() {
        let config = ReleaseConfig {
            version_files: vec!["Cargo.toml".into()],
            packages: vec![PackageConfig {
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
            }],
            ..Default::default()
        };

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
        let config = ReleaseConfig {
            packages: vec![PackageConfig {
                name: "core".into(),
                path: "crates/core".into(),
                tag_prefix: None,
                version_files: vec![],
                changelog: None,
                build_command: None,
                stage_files: vec![],
            }],
            ..Default::default()
        };

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

    #[test]
    fn default_template_parses() {
        let template = default_config_template(&[]);
        let config: ReleaseConfig = serde_yaml_ng::from_str(&template).unwrap();
        let default = ReleaseConfig::default();
        assert_eq!(config.branches, default.branches);
        assert_eq!(config.tag_prefix, default.tag_prefix);
        assert_eq!(config.commit_pattern, default.commit_pattern);
        assert_eq!(config.breaking_section, default.breaking_section);
        assert_eq!(config.types.len(), default.types.len());
        assert!(config.floating_tags);
        assert!(!config.sign_tags);
        assert!(!config.draft);
    }

    #[test]
    fn default_template_with_version_files() {
        let template = default_config_template(&["Cargo.toml".into(), "package.json".into()]);
        let config: ReleaseConfig = serde_yaml_ng::from_str(&template).unwrap();
        assert_eq!(config.version_files, vec!["Cargo.toml", "package.json"]);
    }

    #[test]
    fn default_template_contains_all_fields() {
        let template = default_config_template(&[]);
        for field in [
            "branches",
            "tag_prefix",
            "commit_pattern",
            "breaking_section",
            "misc_section",
            "types",
            "changelog",
            "version_files",
            "version_files_strict",
            "artifacts",
            "floating_tags",
            "build_command",
            "stage_files",
            "prerelease",
            "pre_release_command",
            "post_release_command",
            "sign_tags",
            "draft",
            "release_name_template",
            "hooks",
            "lifecycle",
            "versioning",
            "packages",
        ] {
            assert!(template.contains(field), "template missing field: {field}");
        }
    }

    #[test]
    fn merge_adds_missing_fields() {
        let existing = "tag_prefix: rel-\n";
        let merged = merge_config_yaml(existing).unwrap();
        let config: ReleaseConfig = serde_yaml_ng::from_str(&merged).unwrap();
        // User value preserved
        assert_eq!(config.tag_prefix, "rel-");
        // Defaults filled in
        assert_eq!(config.branches, vec!["main"]);
        assert_eq!(config.breaking_section, "Breaking Changes");
        assert!(!config.types.is_empty());
    }

    #[test]
    fn merge_preserves_user_values() {
        let existing = "branches:\n  - develop\ntag_prefix: release-\nfloating_tags: true\n";
        let merged = merge_config_yaml(existing).unwrap();
        let config: ReleaseConfig = serde_yaml_ng::from_str(&merged).unwrap();
        assert_eq!(config.branches, vec!["develop"]);
        assert_eq!(config.tag_prefix, "release-");
        assert!(config.floating_tags);
    }

    #[test]
    fn merge_nested_changelog() {
        let existing = "changelog:\n  file: CHANGELOG.md\n";
        let merged = merge_config_yaml(existing).unwrap();
        let config: ReleaseConfig = serde_yaml_ng::from_str(&merged).unwrap();
        assert_eq!(config.changelog.file.as_deref(), Some("CHANGELOG.md"));
        // template field should exist (merged from defaults)
        assert!(config.changelog.template.is_none());
    }

    #[test]
    fn lifecycle_step_roundtrip() {
        let step = LifecycleStep {
            name: "lint".into(),
            when: LifecycleEvent::PreRelease,
            run: "cargo clippy".into(),
        };
        let yaml = serde_yaml_ng::to_string(&step).unwrap();
        assert!(yaml.contains("pre_release"));
        let parsed: LifecycleStep = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed, step);
    }

    #[test]
    fn lifecycle_config_parses_from_yaml() {
        let yaml = r#"
lifecycle:
  - name: test
    when: pre_release
    run: "cargo test"
  - name: audit
    when: post_bump
    run: "cargo audit"
  - name: verify
    when: post_build
    run: "./scripts/verify.sh"
  - name: notify
    when: post_release
    run: "./scripts/notify.sh"
"#;
        let config: ReleaseConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.lifecycle.len(), 4);
        assert_eq!(config.lifecycle[0].name, "test");
        assert_eq!(config.lifecycle[0].when, LifecycleEvent::PreRelease);
        assert_eq!(config.lifecycle[1].when, LifecycleEvent::PostBump);
        assert_eq!(config.lifecycle[2].when, LifecycleEvent::PostBuild);
        assert_eq!(config.lifecycle[3].when, LifecycleEvent::PostRelease);
    }

    #[test]
    fn lifecycle_not_serialized_when_empty() {
        let config = ReleaseConfig::default();
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        assert!(!yaml.contains("lifecycle"));
    }

    #[test]
    fn versioning_mode_defaults_to_independent() {
        let config = ReleaseConfig::default();
        assert_eq!(config.versioning, VersioningMode::Independent);
    }

    #[test]
    fn versioning_mode_roundtrip() {
        for (mode, label) in [
            (VersioningMode::Independent, "independent"),
            (VersioningMode::Fixed, "fixed"),
        ] {
            let yaml = serde_yaml_ng::to_string(&mode).unwrap();
            assert!(yaml.contains(label));
            let parsed: VersioningMode = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(parsed, mode);
        }
    }

    #[test]
    fn load_yaml_with_versioning_fixed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            r#"
versioning: fixed
packages:
  - name: core
    path: crates/core
    version_files:
      - crates/core/Cargo.toml
  - name: cli
    path: crates/cli
    version_files:
      - crates/cli/Cargo.toml
"#,
        )
        .unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.versioning, VersioningMode::Fixed);
        assert_eq!(config.packages.len(), 2);
    }

    #[test]
    fn resolve_fixed_collects_all_version_files() {
        let config = ReleaseConfig {
            version_files: vec!["Cargo.toml".into()],
            packages: vec![
                PackageConfig {
                    name: "core".into(),
                    path: "crates/core".into(),
                    tag_prefix: Some("core/v".into()),
                    version_files: vec!["crates/core/Cargo.toml".into()],
                    changelog: None,
                    build_command: None,
                    stage_files: vec!["crates/core/Cargo.lock".into()],
                },
                PackageConfig {
                    name: "cli".into(),
                    path: "crates/cli".into(),
                    tag_prefix: None,
                    version_files: vec!["crates/cli/Cargo.toml".into()],
                    changelog: None,
                    build_command: None,
                    stage_files: vec![],
                },
            ],
            versioning: VersioningMode::Fixed,
            ..Default::default()
        };

        let resolved = config.resolve_fixed();
        // Uses root tag_prefix, not per-package
        assert_eq!(resolved.tag_prefix, "v");
        // No path filter
        assert!(resolved.path_filter.is_none());
        // Collects version files from root + all packages
        assert!(resolved.version_files.contains(&"Cargo.toml".to_string()));
        assert!(resolved
            .version_files
            .contains(&"crates/core/Cargo.toml".to_string()));
        assert!(resolved
            .version_files
            .contains(&"crates/cli/Cargo.toml".to_string()));
        // Collects stage files
        assert!(resolved
            .stage_files
            .contains(&"crates/core/Cargo.lock".to_string()));
        // Packages cleared
        assert!(resolved.packages.is_empty());
    }

    #[test]
    fn resolve_fixed_deduplicates() {
        let config = ReleaseConfig {
            version_files: vec!["Cargo.toml".into()],
            packages: vec![PackageConfig {
                name: "core".into(),
                path: "crates/core".into(),
                tag_prefix: None,
                version_files: vec!["Cargo.toml".into()], // duplicate of root
                changelog: None,
                build_command: None,
                stage_files: vec![],
            }],
            versioning: VersioningMode::Fixed,
            ..Default::default()
        };

        let resolved = config.resolve_fixed();
        let cargo_count = resolved
            .version_files
            .iter()
            .filter(|f| *f == "Cargo.toml")
            .count();
        assert_eq!(cargo_count, 1);
    }
}
