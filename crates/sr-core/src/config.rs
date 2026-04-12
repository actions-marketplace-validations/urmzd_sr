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

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Root configuration. Three top-level concerns:
/// - `commit` — how commits are parsed
/// - `release` — how releases are cut
/// - `hooks` — what runs at each lifecycle event
///
/// ```yaml
/// commit:
///   types: [...]
///   pattern: '...'
///
/// release:
///   branches: [main]
///   tag_prefix: "v"
///   version_files: [Cargo.toml]
///   channels:
///     canary: { prerelease: canary }
///     stable: {}
///
/// hooks:
///   pre_commit: ["cargo fmt --check"]
///   pre_release: ["cargo test --workspace"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub commit: CommitConfig,
    pub release: ReleaseConfig,
    pub hooks: HooksConfig,
    /// Monorepo packages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<PackageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            commit: CommitConfig::default(),
            release: ReleaseConfig::default(),
            hooks: HooksConfig::default(),
            packages: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Commit config
// ---------------------------------------------------------------------------

/// How commits are parsed and classified.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CommitConfig {
    /// Regex for parsing conventional commits.
    pub pattern: String,
    /// Changelog section heading for breaking changes.
    pub breaking_section: String,
    /// Fallback changelog section for unrecognised commit types.
    pub misc_section: String,
    /// Commit type definitions.
    pub types: Vec<CommitType>,
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            pattern: DEFAULT_COMMIT_PATTERN.into(),
            breaking_section: "Breaking Changes".into(),
            misc_section: "Miscellaneous".into(),
            types: default_commit_types(),
        }
    }
}

// ---------------------------------------------------------------------------
// Release config
// ---------------------------------------------------------------------------

/// How releases are cut — versioning, changelog, tags, artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReleaseConfig {
    /// Branches that trigger releases.
    pub branches: Vec<String>,
    /// Prefix for git tags (e.g. "v" → "v1.2.0").
    pub tag_prefix: String,
    /// Changelog configuration.
    pub changelog: ChangelogConfig,
    /// Manifest files to bump (auto-detected if empty).
    pub version_files: Vec<String>,
    /// Fail on unsupported version file formats.
    pub version_files_strict: bool,
    /// Glob patterns for release artifacts.
    pub artifacts: Vec<String>,
    /// Create floating major version tags (e.g. "v3" → latest v3.x.x).
    pub floating_tags: bool,
    /// Additional files to stage in the release commit.
    pub stage_files: Vec<String>,
    /// Pre-release identifier (e.g. "alpha", "rc").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerelease: Option<String>,
    /// Sign tags with GPG/SSH.
    pub sign_tags: bool,
    /// Create GitHub releases as drafts.
    pub draft: bool,
    /// Minijinja template for release name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_name_template: Option<String>,
    /// Versioning strategy for monorepo packages.
    #[serde(default)]
    pub versioning: VersioningMode,
    /// Named release channels for trunk-based promotion.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub channels: BTreeMap<String, ChannelConfig>,
    /// Default channel when no --channel flag given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_channel: Option<String>,
    /// Internal: commits filtered to this path (set by resolve_package).
    #[serde(skip)]
    pub path_filter: Option<String>,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            branches: vec!["main".into()],
            tag_prefix: "v".into(),
            changelog: ChangelogConfig::default(),
            version_files: vec![],
            version_files_strict: false,
            artifacts: vec![],
            floating_tags: true,
            stage_files: vec![],
            prerelease: None,
            sign_tags: false,
            draft: false,
            release_name_template: None,
            versioning: VersioningMode::default(),
            channels: BTreeMap::new(),
            default_channel: None,
            path_filter: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VersioningMode {
    #[default]
    Independent,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub version_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changelog: Option<ChangelogConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ChannelConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prerelease: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreCommit,
    PostCommit,
    PreBranch,
    PostBranch,
    PrePr,
    PostPr,
    PreReview,
    PostReview,
    PreRelease,
    PostRelease,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct HooksConfig {
    pub hooks: BTreeMap<HookEvent, Vec<String>>,
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

// ---------------------------------------------------------------------------
// Config methods
// ---------------------------------------------------------------------------

impl Config {
    /// Find the first config file that exists in the given directory.
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

    /// Resolve a package into a full config by merging package overrides.
    pub fn resolve_package(&self, pkg: &PackageConfig) -> Self {
        let mut config = self.clone();
        config.release.tag_prefix = pkg
            .tag_prefix
            .clone()
            .unwrap_or_else(|| format!("{}/v", pkg.name));
        config.release.path_filter = Some(pkg.path.clone());
        if !pkg.version_files.is_empty() {
            config.release.version_files = pkg.version_files.clone();
        } else if config.release.version_files.is_empty() {
            let detected = detect_version_files(Path::new(&pkg.path));
            if !detected.is_empty() {
                config.release.version_files = detected
                    .into_iter()
                    .map(|f| format!("{}/{f}", pkg.path))
                    .collect();
            }
        }
        if let Some(ref cl) = pkg.changelog {
            config.release.changelog = cl.clone();
        }
        if !pkg.stage_files.is_empty() {
            config.release.stage_files = pkg.stage_files.clone();
        }
        config.packages = vec![];
        config
    }

    /// Resolve all packages for fixed versioning mode.
    pub fn resolve_fixed(&self) -> Self {
        let mut config = self.clone();
        config.release.path_filter = None;

        let mut version_files: Vec<String> = config.release.version_files.clone();
        for pkg in &self.packages {
            if !pkg.version_files.is_empty() {
                version_files.extend(pkg.version_files.clone());
            } else {
                let detected = detect_version_files(Path::new(&pkg.path));
                version_files.extend(detected.into_iter().map(|f| format!("{}/{f}", pkg.path)));
            }
        }
        version_files.sort();
        version_files.dedup();
        config.release.version_files = version_files;

        let mut stage_files = config.release.stage_files.clone();
        for pkg in &self.packages {
            stage_files.extend(pkg.stage_files.clone());
        }
        stage_files.sort();
        stage_files.dedup();
        config.release.stage_files = stage_files;

        config.packages = vec![];
        config
    }

    /// Resolve a named release channel.
    pub fn resolve_channel(&self, name: &str) -> Result<Self, ReleaseError> {
        let channel = self.release.channels.get(name).ok_or_else(|| {
            let available: Vec<&str> = self.release.channels.keys().map(|k| k.as_str()).collect();
            ReleaseError::Config(format!(
                "channel '{name}' not found. Available: {}",
                if available.is_empty() {
                    "(none)".to_string()
                } else {
                    available.join(", ")
                }
            ))
        })?;

        let mut config = self.clone();
        if channel.prerelease.is_some() {
            config.release.prerelease = channel.prerelease.clone();
        }
        if channel.draft {
            config.release.draft = true;
        }
        if !channel.artifacts.is_empty() {
            config.release.artifacts.extend(channel.artifacts.clone());
        }
        Ok(config)
    }

    /// Find a package by name.
    pub fn find_package(&self, name: &str) -> Result<&PackageConfig, ReleaseError> {
        self.packages
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| {
                let available: Vec<&str> = self.packages.iter().map(|p| p.name.as_str()).collect();
                ReleaseError::Config(format!(
                    "package '{name}' not found. Available: {}",
                    if available.is_empty() {
                        "(none)".to_string()
                    } else {
                        available.join(", ")
                    }
                ))
            })
    }
}

// ---------------------------------------------------------------------------
// Template generation
// ---------------------------------------------------------------------------

pub fn default_config_template(version_files: &[String]) -> String {
    let vf = if version_files.is_empty() {
        "  version_files: []\n".to_string()
    } else {
        let mut s = "  version_files:\n".to_string();
        for f in version_files {
            s.push_str(&format!("    - {f}\n"));
        }
        s
    };

    format!(
        r#"# sr configuration
# Full reference: https://github.com/urmzd/sr#configuration

# How commits are parsed and classified.
commit:
  # Regex for parsing conventional commits.
  # Required named groups: type, description. Optional: scope, breaking.
  pattern: '^(?P<type>\w+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?:\s+(?P<description>.+)'

  # Changelog section headings.
  breaking_section: Breaking Changes
  misc_section: Miscellaneous

  # Commit type definitions.
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

# How releases are cut.
release:
  branches:
    - main
  tag_prefix: "v"
  changelog:
    file: CHANGELOG.md
{vf}  version_files_strict: false
  artifacts: []
  floating_tags: true
  stage_files: []
  sign_tags: false
  draft: false
  # prerelease: alpha
  # release_name_template: "{{{{ tag_name }}}}"

  # Release channels for trunk-based promotion.
  # channels:
  #   canary:
  #     prerelease: canary
  #   rc:
  #     prerelease: rc
  #     draft: true
  #   stable: {{}}
  # default_channel: stable

# Lifecycle hooks — shell commands keyed by event.
# Available events: pre_commit, post_commit, pre_branch, post_branch,
#   pre_pr, post_pr, pre_review, post_review, pre_release, post_release.
# Release hooks receive SR_VERSION and SR_TAG env vars.
# hooks:
#   pre_commit:
#     - "cargo fmt --check"
#     - "cargo clippy -- -D warnings"
#   pre_release:
#     - "cargo test --workspace"
#   post_release:
#     - "./scripts/notify-slack.sh"

# Monorepo packages (uncomment and configure if needed).
# packages:
#   - name: core
#     path: crates/core
#     tag_prefix: "core/v"
#     version_files:
#       - crates/core/Cargo.toml
#     stage_files:
#       - crates/core/Cargo.lock
"#
    )
}

/// Merge new default fields into an existing config YAML string.
pub fn merge_config_yaml(existing_yaml: &str) -> Result<String, ReleaseError> {
    let mut existing: serde_yaml_ng::Value = serde_yaml_ng::from_str(existing_yaml)
        .map_err(|e| ReleaseError::Config(format!("failed to parse existing config: {e}")))?;

    let default_config = Config::default();
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

fn deep_merge_value(base: &mut serde_yaml_ng::Value, defaults: &serde_yaml_ng::Value) {
    use serde_yaml_ng::Value;
    if let (Value::Mapping(base_map), Value::Mapping(default_map)) = (base, defaults) {
        for (key, default_val) in default_map {
            match base_map.get_mut(key) {
                Some(existing_val) => {
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

// ---------------------------------------------------------------------------
// Serde for BumpLevel
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_values() {
        let config = Config::default();
        assert_eq!(config.release.branches, vec!["main"]);
        assert_eq!(config.release.tag_prefix, "v");
        assert_eq!(config.commit.pattern, DEFAULT_COMMIT_PATTERN);
        assert_eq!(config.commit.breaking_section, "Breaking Changes");
        assert_eq!(config.commit.misc_section, "Miscellaneous");
        assert!(!config.commit.types.is_empty());
        assert!(!config.release.version_files_strict);
        assert!(config.release.artifacts.is_empty());
        assert!(config.release.floating_tags);
        assert_eq!(config.release.changelog.file.as_deref(), Some("CHANGELOG.md"));
        let refactor = config.commit.types.iter().find(|t| t.name == "refactor").unwrap();
        assert_eq!(refactor.bump, Some(BumpLevel::Patch));
    }

    #[test]
    fn load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.yml");
        let config = Config::load(&path).unwrap();
        assert_eq!(config.release.tag_prefix, "v");
    }

    #[test]
    fn load_nested_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "commit:\n  pattern: custom\nrelease:\n  branches:\n    - develop\n  tag_prefix: release-"
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.release.branches, vec!["develop"]);
        assert_eq!(config.release.tag_prefix, "release-");
        assert_eq!(config.commit.pattern, "custom");
    }

    #[test]
    fn load_partial_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(&path, "release:\n  tag_prefix: rel-\n").unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.release.tag_prefix, "rel-");
        assert_eq!(config.release.branches, vec!["main"]);
        assert_eq!(config.commit.pattern, DEFAULT_COMMIT_PATTERN);
    }

    #[test]
    fn load_yaml_with_packages() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            "packages:\n  - name: core\n    path: crates/core\n    version_files:\n      - crates/core/Cargo.toml\n",
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(config.packages.len(), 1);
        assert_eq!(config.packages[0].name, "core");
    }

    #[test]
    fn resolve_package_defaults() {
        let config = Config {
            packages: vec![PackageConfig {
                name: "core".into(),
                path: "crates/core".into(),
                tag_prefix: None,
                version_files: vec![],
                changelog: None,
                stage_files: vec![],
            }],
            ..Default::default()
        };

        let resolved = config.resolve_package(&config.packages[0]);
        assert_eq!(resolved.release.tag_prefix, "core/v");
        assert_eq!(resolved.release.path_filter.as_deref(), Some("crates/core"));
        assert!(resolved.packages.is_empty());
    }

    #[test]
    fn resolve_package_overrides() {
        let mut config = Config::default();
        config.release.version_files = vec!["Cargo.toml".into()];
        config.packages = vec![PackageConfig {
            name: "cli".into(),
            path: "crates/cli".into(),
            tag_prefix: Some("cli-v".into()),
            version_files: vec!["crates/cli/Cargo.toml".into()],
            changelog: Some(ChangelogConfig {
                file: Some("crates/cli/CHANGELOG.md".into()),
                template: None,
            }),
            stage_files: vec!["crates/cli/Cargo.lock".into()],
        }];

        let resolved = config.resolve_package(&config.packages[0]);
        assert_eq!(resolved.release.tag_prefix, "cli-v");
        assert_eq!(resolved.release.version_files, vec!["crates/cli/Cargo.toml"]);
        assert_eq!(resolved.release.stage_files, vec!["crates/cli/Cargo.lock"]);
    }

    #[test]
    fn find_package_not_found() {
        let config = Config::default();
        let err = config.find_package("nonexistent").unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn resolve_channel() {
        let mut config = Config::default();
        config.release.channels.insert(
            "canary".into(),
            ChannelConfig {
                prerelease: Some("canary".into()),
                ..Default::default()
            },
        );

        let resolved = config.resolve_channel("canary").unwrap();
        assert_eq!(resolved.release.prerelease.as_deref(), Some("canary"));
    }

    #[test]
    fn resolve_channel_not_found() {
        let config = Config::default();
        assert!(config.resolve_channel("missing").is_err());
    }

    #[test]
    fn hook_event_roundtrip() {
        let mut hooks = BTreeMap::new();
        hooks.insert(HookEvent::PreRelease, vec!["cargo test".to_string()]);
        let config = HooksConfig { hooks };
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        assert!(yaml.contains("pre_release"));
        let parsed: HooksConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        assert!(parsed.hooks.contains_key(&HookEvent::PreRelease));
    }

    #[test]
    fn default_template_parses() {
        let template = default_config_template(&[]);
        let config: Config = serde_yaml_ng::from_str(&template).unwrap();
        assert_eq!(config.release.branches, vec!["main"]);
        assert_eq!(config.release.tag_prefix, "v");
        assert!(config.release.floating_tags);
    }

    #[test]
    fn default_template_with_version_files() {
        let template = default_config_template(&["Cargo.toml".into(), "package.json".into()]);
        let config: Config = serde_yaml_ng::from_str(&template).unwrap();
        assert_eq!(config.release.version_files, vec!["Cargo.toml", "package.json"]);
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
}
