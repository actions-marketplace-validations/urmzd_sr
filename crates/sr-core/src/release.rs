use std::fs;
use std::path::Path;

use semver::Version;
use serde::Serialize;

use crate::changelog::{ChangelogEntry, ChangelogFormatter};
use crate::commit::{CommitParser, ConventionalCommit, DefaultCommitClassifier};
use crate::config::{Config, PackageConfig};
use crate::error::ReleaseError;
use crate::git::GitRepository;
use crate::version::{BumpLevel, apply_bump, apply_prerelease_bump, determine_bump};
use crate::version_files::{bump_version_file, discover_lock_files, is_supported_version_file};

/// The computed plan for a release, before execution.
#[derive(Debug, Serialize)]
pub struct ReleasePlan {
    pub current_version: Option<Version>,
    pub next_version: Version,
    pub bump: BumpLevel,
    pub commits: Vec<ConventionalCommit>,
    pub tag_name: String,
    pub floating_tag_name: Option<String>,
    pub prerelease: bool,
}

/// Orchestrates the release flow.
pub trait ReleaseStrategy: Send + Sync {
    /// Plan the release without executing it.
    fn plan(&self) -> Result<ReleasePlan, ReleaseError>;

    /// Execute the release.
    fn execute(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError>;
}

/// Abstraction over a remote VCS provider (e.g. GitHub, GitLab).
pub trait VcsProvider: Send + Sync {
    /// Create a release on the remote VCS.
    fn create_release(
        &self,
        tag: &str,
        name: &str,
        body: &str,
        prerelease: bool,
        draft: bool,
    ) -> Result<String, ReleaseError>;

    /// Generate a compare URL between two refs.
    fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError>;

    /// Check if a release already exists for the given tag.
    fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError>;

    /// Delete a release by tag.
    fn delete_release(&self, tag: &str) -> Result<(), ReleaseError>;

    /// Return the base URL of the repository (e.g. `https://github.com/owner/repo`).
    fn repo_url(&self) -> Option<String> {
        None
    }

    /// Update an existing release (name and body) using PATCH semantics,
    /// preserving any previously uploaded assets.
    fn update_release(
        &self,
        _tag: &str,
        _name: &str,
        _body: &str,
        _prerelease: bool,
        _draft: bool,
    ) -> Result<String, ReleaseError> {
        Err(ReleaseError::Vcs(
            "update_release not implemented for this provider".into(),
        ))
    }

    /// Upload asset files to an existing release identified by tag.
    fn upload_assets(&self, _tag: &str, _files: &[&str]) -> Result<(), ReleaseError> {
        Ok(())
    }

    /// Verify that a release exists and is in the expected state after creation.
    fn verify_release(&self, _tag: &str) -> Result<(), ReleaseError> {
        Ok(())
    }
}

/// A no-op VcsProvider that silently succeeds. Used when no remote VCS
/// (e.g. GitHub) is configured.
pub struct NoopVcsProvider;

impl VcsProvider for NoopVcsProvider {
    fn create_release(
        &self,
        _tag: &str,
        _name: &str,
        _body: &str,
        _prerelease: bool,
        _draft: bool,
    ) -> Result<String, ReleaseError> {
        Ok(String::new())
    }

    fn compare_url(&self, _base: &str, _head: &str) -> Result<String, ReleaseError> {
        Ok(String::new())
    }

    fn release_exists(&self, _tag: &str) -> Result<bool, ReleaseError> {
        Ok(false)
    }

    fn delete_release(&self, _tag: &str) -> Result<(), ReleaseError> {
        Ok(())
    }
}

/// Concrete release strategy implementing the trunk-based release flow.
pub struct TrunkReleaseStrategy<G, V, C, F> {
    pub git: G,
    pub vcs: V,
    pub parser: C,
    pub formatter: F,
    pub config: Config,
    /// When true, re-release the current tag if HEAD is at the latest tag.
    pub force: bool,
    /// Pre-release identifier resolved from the active channel (None = stable).
    pub prerelease_id: Option<String>,
    /// Whether the GitHub release should be created as a draft.
    pub draft: bool,
}

impl<G, V, C, F> TrunkReleaseStrategy<G, V, C, F>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
{
    fn format_changelog(&self, plan: &ReleasePlan) -> Result<String, ReleaseError> {
        let today = today_string();
        let compare_url = match &plan.current_version {
            Some(v) => {
                let base = format!("{}{v}", self.config.git.tag_prefix);
                self.vcs
                    .compare_url(&base, &plan.tag_name)
                    .ok()
                    .filter(|s| !s.is_empty())
            }
            None => None,
        };
        let entry = ChangelogEntry {
            version: plan.next_version.to_string(),
            date: today,
            commits: plan.commits.clone(),
            compare_url,
            repo_url: self.vcs.repo_url(),
        };
        self.formatter.format(&[entry])
    }

    /// Render the release name from the configured template, or fall back to the tag name.
    fn release_name(&self, plan: &ReleasePlan) -> String {
        if let Some(ref template_str) = self.config.vcs.github.release_name_template {
            let mut env = minijinja::Environment::new();
            if env.add_template("release_name", template_str).is_ok()
                && let Ok(tmpl) = env.get_template("release_name")
                && let Ok(rendered) = tmpl.render(minijinja::context! {
                    version => plan.next_version.to_string(),
                    tag_name => &plan.tag_name,
                    tag_prefix => &self.config.git.tag_prefix,
                })
            {
                return rendered;
            }
            eprintln!("warning: invalid release_name_template, falling back to tag name");
        }
        plan.tag_name.clone()
    }

    /// Return the active package for a single-package release (the root package or the only one).
    /// Returns the root package (".") if present, otherwise the first package.
    fn active_package(&self) -> Option<&PackageConfig> {
        self.config
            .packages
            .iter()
            .find(|p| p.path == ".")
            .or_else(|| self.config.packages.first())
    }
}

impl<G, V, C, F> ReleaseStrategy for TrunkReleaseStrategy<G, V, C, F>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
{
    fn plan(&self) -> Result<ReleasePlan, ReleaseError> {
        let is_prerelease = self.prerelease_id.is_some();

        // For stable releases, find the latest stable tag (skip pre-release tags).
        // For pre-releases, find the latest tag of any kind to determine commits since.
        let all_tags = self.git.all_tags(&self.config.git.tag_prefix)?;
        let latest_stable = all_tags.iter().rev().find(|t| t.version.pre.is_empty());
        let latest_any = all_tags.last();

        // Use the latest tag (any kind) for commit range, but the latest stable for base version
        let tag_info = if is_prerelease {
            latest_any
        } else {
            latest_stable.or(latest_any)
        };

        let (current_version, from_sha) = match tag_info {
            Some(info) => (Some(info.version.clone()), Some(info.sha.as_str())),
            None => (None, None),
        };

        let default_pkg = PackageConfig::default();
        let pkg = self.active_package().unwrap_or(&default_pkg);
        let path_filter = if pkg.path != "." {
            Some(pkg.path.as_str())
        } else {
            None
        };

        let raw_commits = if let Some(path) = path_filter {
            self.git.commits_since_in_path(from_sha, path)?
        } else {
            self.git.commits_since(from_sha)?
        };

        if raw_commits.is_empty() {
            // Force mode: re-release if HEAD is exactly at the latest tag
            if self.force
                && let Some(info) = tag_info
            {
                let head = self.git.head_sha()?;
                if head == info.sha {
                    let floating_tag_name = if self.config.git.floating_tag {
                        Some(format!(
                            "{}{}",
                            self.config.git.tag_prefix, info.version.major
                        ))
                    } else {
                        None
                    };
                    return Ok(ReleasePlan {
                        current_version: Some(info.version.clone()),
                        next_version: info.version.clone(),
                        bump: BumpLevel::Patch,
                        commits: vec![],
                        tag_name: info.name.clone(),
                        floating_tag_name,
                        prerelease: is_prerelease,
                    });
                }
            }
            let (tag, sha) = match tag_info {
                Some(info) => (info.name.clone(), info.sha.clone()),
                None => ("(none)".into(), "(none)".into()),
            };
            return Err(ReleaseError::NoCommits { tag, sha });
        }

        let skip_patterns = &self.config.git.skip_patterns;
        let conventional_commits: Vec<ConventionalCommit> = raw_commits
            .iter()
            .filter(|c| !c.message.starts_with("chore(release):"))
            .filter(|c| !skip_patterns.iter().any(|p| c.message.contains(p.as_str())))
            .filter_map(|c| self.parser.parse(c).ok())
            .collect();

        let classifier = DefaultCommitClassifier::new(self.config.commit.types.into_commit_types());
        let tag_for_err = tag_info
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "(none)".into());
        let commit_count = conventional_commits.len();
        let bump = match determine_bump(&conventional_commits, &classifier) {
            Some(b) => b,
            None if self.force => BumpLevel::Patch,
            None => {
                return Err(ReleaseError::NoBump {
                    tag: tag_for_err,
                    commit_count,
                });
            }
        };

        // For pre-releases, base the version on the latest *stable* tag
        let base_version = if is_prerelease {
            latest_stable
                .map(|t| t.version.clone())
                .or(current_version.clone())
                .unwrap_or(Version::new(0, 0, 0))
        } else {
            current_version.clone().unwrap_or(Version::new(0, 0, 0))
        };

        // v0 protection: downshift Major → Minor when version is 0.x.y
        // to prevent accidentally bumping to v1. Disable with git.v0_protection: false.
        let bump =
            if base_version.major == 0 && bump == BumpLevel::Major && self.config.git.v0_protection
            {
                eprintln!(
                    "v0 protection: breaking change detected at v{base_version}, \
                     downshifting major → minor (set git.v0_protection: false to bump to v1)"
                );
                BumpLevel::Minor
            } else {
                bump
            };

        let next_version = if let Some(ref prerelease_id) = self.prerelease_id {
            let existing_versions: Vec<Version> =
                all_tags.iter().map(|t| t.version.clone()).collect();
            apply_prerelease_bump(&base_version, bump, prerelease_id, &existing_versions)
        } else {
            apply_bump(&base_version, bump)
        };

        let tag_name = format!("{}{next_version}", self.config.git.tag_prefix);

        // Don't update floating tags for pre-releases
        let floating_tag_name = if self.config.git.floating_tag && !is_prerelease {
            Some(format!(
                "{}{}",
                self.config.git.tag_prefix, next_version.major
            ))
        } else {
            None
        };

        Ok(ReleasePlan {
            current_version,
            next_version,
            bump,
            commits: conventional_commits,
            tag_name,
            floating_tag_name,
            prerelease: is_prerelease,
        })
    }

    fn execute(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError> {
        let version_str = plan.next_version.to_string();

        // 1. Run pre_release hooks
        let env = release_env(&version_str, &plan.tag_name);
        let env_refs: Vec<(&str, &str)> =
            env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        if !dry_run
            && let Some(pkg) = self.active_package()
            && let Some(ref hooks) = pkg.hooks
        {
            crate::hooks::run_pre_release(hooks, &env_refs)?;
        }

        // 2. Generate changelog
        let changelog_body = self.format_changelog(plan)?;

        // 3. Bump, write changelog, stage, commit
        self.bump_and_build(plan, &version_str, &changelog_body, dry_run)?;

        // 4. Create and push tags
        self.create_and_push_tags(plan, &changelog_body, dry_run)?;

        // 5. GitHub release
        self.create_or_update_release(plan, &changelog_body, dry_run)?;
        self.upload_artifacts(plan, dry_run)?;
        self.verify_release_exists(plan, dry_run)?;

        // 6. Run post_release hooks
        if !dry_run
            && let Some(pkg) = self.active_package()
            && let Some(ref hooks) = pkg.hooks
        {
            crate::hooks::run_post_release(hooks, &env_refs)?;
        }

        if dry_run {
            eprintln!("[dry-run] Changelog:\n{changelog_body}");
        } else {
            eprintln!("Released {}", plan.tag_name);
        }
        Ok(())
    }
}

/// Build release env vars as owned strings.
fn release_env(version: &str, tag: &str) -> Vec<(String, String)> {
    vec![
        ("SR_VERSION".into(), version.into()),
        ("SR_TAG".into(), tag.into()),
    ]
}

impl<G, V, C, F> TrunkReleaseStrategy<G, V, C, F>
where
    G: GitRepository,
    V: VcsProvider,
    C: CommitParser,
    F: ChangelogFormatter,
{
    fn bump_and_build(
        &self,
        plan: &ReleasePlan,
        version_str: &str,
        changelog_body: &str,
        dry_run: bool,
    ) -> Result<(), ReleaseError> {
        let default_pkg = PackageConfig::default();
        let pkg = self.active_package().unwrap_or(&default_pkg);
        let version_files = self.config.version_files_for(pkg);
        let version_files_strict = pkg.version_files_strict;
        let stage_files = &pkg.stage_files;
        let changelog_file = self.config.changelog_for(pkg).file.clone();

        if dry_run {
            for file in &version_files {
                let filename = Path::new(file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if is_supported_version_file(filename) {
                    eprintln!("[dry-run] Would bump version in: {file}");
                } else if version_files_strict {
                    return Err(ReleaseError::VersionBump(format!(
                        "unsupported version file: {filename}"
                    )));
                } else {
                    eprintln!("[dry-run] warning: unsupported version file, would skip: {file}");
                }
            }
            if !stage_files.is_empty() {
                eprintln!(
                    "[dry-run] Would stage additional files: {}",
                    stage_files.join(", ")
                );
            }
            return Ok(());
        }

        let files_to_stage = self.execute_mutations(
            version_str,
            changelog_body,
            &version_files,
            &changelog_file,
            version_files_strict,
        )?;

        // Resolve stage_files globs and collect all paths to stage
        let mut paths_to_stage: Vec<String> = Vec::new();
        if let Some(ref cf) = changelog_file {
            paths_to_stage.push(cf.clone());
        }
        for file in &files_to_stage {
            paths_to_stage.push(file.clone());
        }
        if !stage_files.is_empty() {
            let extra = resolve_globs(stage_files).map_err(ReleaseError::Config)?;
            paths_to_stage.extend(extra);
        }
        if !paths_to_stage.is_empty() {
            let refs: Vec<&str> = paths_to_stage.iter().map(|s| s.as_str()).collect();
            let commit_msg = format!("chore(release): {} [skip ci]", plan.tag_name);
            self.git.stage_and_commit(&refs, &commit_msg)?;
        }
        Ok(())
    }

    fn create_and_push_tags(
        &self,
        plan: &ReleasePlan,
        changelog_body: &str,
        dry_run: bool,
    ) -> Result<(), ReleaseError> {
        if dry_run {
            let sign_label = if self.config.git.sign_tags {
                " (signed)"
            } else {
                ""
            };
            eprintln!("[dry-run] Would create tag: {}{sign_label}", plan.tag_name);
            eprintln!("[dry-run] Would push commit and tag: {}", plan.tag_name);
            if let Some(ref floating) = plan.floating_tag_name {
                eprintln!("[dry-run] Would create/update floating tag: {floating}");
                eprintln!("[dry-run] Would force-push floating tag: {floating}");
            }
            return Ok(());
        }

        // Create tag (skip if it already exists locally)
        if !self.git.tag_exists(&plan.tag_name)? {
            let tag_message = format!("{}\n\n{}", plan.tag_name, changelog_body);
            self.git
                .create_tag(&plan.tag_name, &tag_message, self.config.git.sign_tags)?;
        }

        // Push commit (safe to re-run — no-op if up to date)
        self.git.push()?;

        // Push tag (skip if tag already exists on remote)
        if !self.git.remote_tag_exists(&plan.tag_name)? {
            self.git.push_tag(&plan.tag_name)?;
        }

        // Force-create and force-push floating tag (e.g. v3)
        if let Some(ref floating) = plan.floating_tag_name {
            self.git.force_create_tag(floating)?;
            self.git.force_push_tag(floating)?;
        }
        Ok(())
    }

    fn create_or_update_release(
        &self,
        plan: &ReleasePlan,
        changelog_body: &str,
        dry_run: bool,
    ) -> Result<(), ReleaseError> {
        if dry_run {
            let draft_label = if self.draft { " (draft)" } else { "" };
            let release_name = self.release_name(plan);
            eprintln!(
                "[dry-run] Would create GitHub release \"{release_name}\" for {}{draft_label}",
                plan.tag_name
            );
            return Ok(());
        }

        let release_name = self.release_name(plan);
        if self.vcs.release_exists(&plan.tag_name)? {
            self.vcs.update_release(
                &plan.tag_name,
                &release_name,
                changelog_body,
                plan.prerelease,
                self.draft,
            )?;
        } else {
            self.vcs.create_release(
                &plan.tag_name,
                &release_name,
                changelog_body,
                plan.prerelease,
                self.draft,
            )?;
        }
        Ok(())
    }

    fn upload_artifacts(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError> {
        let all_artifacts = self.config.all_artifacts();
        if all_artifacts.is_empty() {
            return Ok(());
        }

        let resolved = resolve_globs(&all_artifacts).map_err(ReleaseError::Vcs)?;

        if dry_run {
            if resolved.is_empty() {
                eprintln!("[dry-run] Artifact patterns matched no files");
            } else {
                eprintln!("[dry-run] Would upload {} artifact(s):", resolved.len());
                for f in &resolved {
                    eprintln!("[dry-run]   {f}");
                }
            }
            return Ok(());
        }

        if !resolved.is_empty() {
            let file_refs: Vec<&str> = resolved.iter().map(|s| s.as_str()).collect();
            self.vcs.upload_assets(&plan.tag_name, &file_refs)?;
            eprintln!(
                "Uploaded {} artifact(s) to {}",
                resolved.len(),
                plan.tag_name
            );
        }
        Ok(())
    }

    fn verify_release_exists(&self, plan: &ReleasePlan, dry_run: bool) -> Result<(), ReleaseError> {
        if dry_run {
            eprintln!("[dry-run] Would verify release: {}", plan.tag_name);
            return Ok(());
        }

        if let Err(e) = self.vcs.verify_release(&plan.tag_name) {
            eprintln!("warning: post-release verification failed: {e}");
            eprintln!(
                "  The tag {} was pushed but the GitHub release may be incomplete.",
                plan.tag_name
            );
            eprintln!("  Re-run with --force to retry.");
        }
        Ok(())
    }

    /// Bump version files and write changelog.
    /// Returns the list of bumped files on success.
    fn execute_mutations(
        &self,
        version_str: &str,
        changelog_body: &str,
        version_files: &[String],
        changelog_file: &Option<String>,
        version_files_strict: bool,
    ) -> Result<Vec<String>, ReleaseError> {
        let mut files_to_stage: Vec<String> = Vec::new();
        for file in version_files {
            match bump_version_file(Path::new(file), version_str) {
                Ok(extra) => {
                    files_to_stage.push(file.clone());
                    for extra_path in extra {
                        files_to_stage.push(extra_path.to_string_lossy().into_owned());
                    }
                }
                Err(e) if !version_files_strict => {
                    eprintln!("warning: {e} — skipping {file}");
                }
                Err(e) => return Err(e),
            }
        }

        // Auto-discover and stage lock files associated with bumped manifests
        for lock_file in discover_lock_files(&files_to_stage) {
            let lock_str = lock_file.to_string_lossy().into_owned();
            if !files_to_stage.contains(&lock_str) {
                files_to_stage.push(lock_str);
            }
        }

        // Write changelog file if configured
        if let Some(cf) = changelog_file {
            let path = Path::new(cf);
            let existing = if path.exists() {
                fs::read_to_string(path).map_err(|e| ReleaseError::Changelog(e.to_string()))?
            } else {
                String::new()
            };
            let new_content = if existing.is_empty() {
                format!("# Changelog\n\n{changelog_body}\n")
            } else {
                match existing.find("\n\n") {
                    Some(pos) => {
                        let (header, rest) = existing.split_at(pos);
                        format!("{header}\n\n{changelog_body}\n{rest}")
                    }
                    None => format!("{existing}\n\n{changelog_body}\n"),
                }
            };
            fs::write(path, new_content).map_err(|e| ReleaseError::Changelog(e.to_string()))?;
        }

        Ok(files_to_stage)
    }
}

/// Resolve glob patterns into a deduplicated, sorted list of file paths.
fn resolve_globs(patterns: &[String]) -> Result<Vec<String>, String> {
    let mut files = std::collections::BTreeSet::new();
    for pattern in patterns {
        let paths =
            glob::glob(pattern).map_err(|e| format!("invalid glob pattern '{pattern}': {e}"))?;
        for entry in paths {
            match entry {
                Ok(path) if path.is_file() => {
                    files.insert(path.to_string_lossy().into_owned());
                }
                Ok(_) => {}
                Err(e) => {
                    return Err(format!("glob error for pattern '{pattern}': {e}"));
                }
            }
        }
    }
    Ok(files.into_iter().collect())
}

pub fn today_string() -> String {
    // Portable date calculation from UNIX epoch (no external deps or subprocess).
    // Uses Howard Hinnant's civil_from_days algorithm.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let z = secs / 86400 + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::changelog::DefaultChangelogFormatter;
    use crate::commit::{Commit, TypedCommitParser};
    use crate::config::{
        ChangelogConfig, Config, GitConfig, PackageConfig, default_changelog_groups,
    };
    use crate::git::{GitRepository, TagInfo};

    // --- Fakes ---

    struct FakeGit {
        tags: Vec<TagInfo>,
        commits: Vec<Commit>,
        /// Commits returned when path filtering is active (None = fall back to `commits`).
        path_commits: Option<Vec<Commit>>,
        head: String,
        created_tags: Mutex<Vec<String>>,
        pushed_tags: Mutex<Vec<String>>,
        committed: Mutex<Vec<(Vec<String>, String)>>,
        push_count: Mutex<u32>,
        force_created_tags: Mutex<Vec<String>>,
        force_pushed_tags: Mutex<Vec<String>>,
    }

    impl FakeGit {
        fn new(tags: Vec<TagInfo>, commits: Vec<Commit>) -> Self {
            let head = tags
                .last()
                .map(|t| t.sha.clone())
                .unwrap_or_else(|| "0".repeat(40));
            Self {
                tags,
                commits,
                path_commits: None,
                head,
                created_tags: Mutex::new(Vec::new()),
                pushed_tags: Mutex::new(Vec::new()),
                committed: Mutex::new(Vec::new()),
                push_count: Mutex::new(0),
                force_created_tags: Mutex::new(Vec::new()),
                force_pushed_tags: Mutex::new(Vec::new()),
            }
        }
    }

    impl GitRepository for FakeGit {
        fn latest_tag(&self, _prefix: &str) -> Result<Option<TagInfo>, ReleaseError> {
            Ok(self.tags.last().cloned())
        }

        fn commits_since(&self, _from: Option<&str>) -> Result<Vec<Commit>, ReleaseError> {
            Ok(self.commits.clone())
        }

        fn create_tag(&self, name: &str, _message: &str, _sign: bool) -> Result<(), ReleaseError> {
            self.created_tags.lock().unwrap().push(name.to_string());
            Ok(())
        }

        fn push_tag(&self, name: &str) -> Result<(), ReleaseError> {
            self.pushed_tags.lock().unwrap().push(name.to_string());
            Ok(())
        }

        fn stage_and_commit(&self, paths: &[&str], message: &str) -> Result<bool, ReleaseError> {
            self.committed.lock().unwrap().push((
                paths.iter().map(|s| s.to_string()).collect(),
                message.to_string(),
            ));
            Ok(true)
        }

        fn push(&self) -> Result<(), ReleaseError> {
            *self.push_count.lock().unwrap() += 1;
            Ok(())
        }

        fn tag_exists(&self, name: &str) -> Result<bool, ReleaseError> {
            Ok(self
                .created_tags
                .lock()
                .unwrap()
                .contains(&name.to_string()))
        }

        fn remote_tag_exists(&self, name: &str) -> Result<bool, ReleaseError> {
            Ok(self.pushed_tags.lock().unwrap().contains(&name.to_string()))
        }

        fn all_tags(&self, _prefix: &str) -> Result<Vec<TagInfo>, ReleaseError> {
            Ok(self.tags.clone())
        }

        fn commits_between(
            &self,
            _from: Option<&str>,
            _to: &str,
        ) -> Result<Vec<Commit>, ReleaseError> {
            Ok(self.commits.clone())
        }

        fn tag_date(&self, _tag_name: &str) -> Result<String, ReleaseError> {
            Ok("2026-01-01".into())
        }

        fn force_create_tag(&self, name: &str) -> Result<(), ReleaseError> {
            self.force_created_tags
                .lock()
                .unwrap()
                .push(name.to_string());
            Ok(())
        }

        fn force_push_tag(&self, name: &str) -> Result<(), ReleaseError> {
            self.force_pushed_tags
                .lock()
                .unwrap()
                .push(name.to_string());
            Ok(())
        }

        fn head_sha(&self) -> Result<String, ReleaseError> {
            Ok(self.head.clone())
        }

        fn commits_since_in_path(
            &self,
            _from: Option<&str>,
            _path: &str,
        ) -> Result<Vec<Commit>, ReleaseError> {
            Ok(self
                .path_commits
                .clone()
                .unwrap_or_else(|| self.commits.clone()))
        }
    }

    struct FakeVcs {
        releases: Mutex<Vec<(String, String)>>,
        deleted_releases: Mutex<Vec<String>>,
        uploaded_assets: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl FakeVcs {
        fn new() -> Self {
            Self {
                releases: Mutex::new(Vec::new()),
                deleted_releases: Mutex::new(Vec::new()),
                uploaded_assets: Mutex::new(Vec::new()),
            }
        }
    }

    impl VcsProvider for FakeVcs {
        fn create_release(
            &self,
            tag: &str,
            _name: &str,
            body: &str,
            _prerelease: bool,
            _draft: bool,
        ) -> Result<String, ReleaseError> {
            self.releases
                .lock()
                .unwrap()
                .push((tag.to_string(), body.to_string()));
            Ok(format!("https://github.com/test/release/{tag}"))
        }

        fn compare_url(&self, base: &str, head: &str) -> Result<String, ReleaseError> {
            Ok(format!("https://github.com/test/compare/{base}...{head}"))
        }

        fn release_exists(&self, tag: &str) -> Result<bool, ReleaseError> {
            Ok(self.releases.lock().unwrap().iter().any(|(t, _)| t == tag))
        }

        fn delete_release(&self, tag: &str) -> Result<(), ReleaseError> {
            self.deleted_releases.lock().unwrap().push(tag.to_string());
            self.releases.lock().unwrap().retain(|(t, _)| t != tag);
            Ok(())
        }

        fn update_release(
            &self,
            tag: &str,
            _name: &str,
            body: &str,
            _prerelease: bool,
            _draft: bool,
        ) -> Result<String, ReleaseError> {
            let mut releases = self.releases.lock().unwrap();
            if let Some(entry) = releases.iter_mut().find(|(t, _)| t == tag) {
                entry.1 = body.to_string();
            }
            Ok(format!("https://github.com/test/release/{tag}"))
        }

        fn upload_assets(&self, tag: &str, files: &[&str]) -> Result<(), ReleaseError> {
            self.uploaded_assets.lock().unwrap().push((
                tag.to_string(),
                files.iter().map(|s| s.to_string()).collect(),
            ));
            Ok(())
        }

        fn repo_url(&self) -> Option<String> {
            Some("https://github.com/test/repo".into())
        }
    }

    // --- Helpers ---

    type TestStrategy =
        TrunkReleaseStrategy<FakeGit, FakeVcs, TypedCommitParser, DefaultChangelogFormatter>;

    /// Build a Config with changelog file disabled and a dummy version file,
    /// so tests don't pollute the real CHANGELOG.md or auto-detect and bump
    /// the actual Cargo.toml of whichever crate is running the tests.
    fn test_config() -> Config {
        Config {
            changelog: ChangelogConfig {
                file: None,
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: ".".into(),
                version_files: vec!["__sr_test_dummy_no_bump__".into()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Build a Config with custom git settings (still isolated from real files).
    fn config_with_git(git: GitConfig) -> Config {
        Config {
            git,
            changelog: ChangelogConfig {
                file: None,
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: ".".into(),
                version_files: vec!["__sr_test_dummy_no_bump__".into()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn make_strategy(tags: Vec<TagInfo>, commits: Vec<Commit>, config: Config) -> TestStrategy {
        TrunkReleaseStrategy {
            git: FakeGit::new(tags, commits),
            vcs: FakeVcs::new(),
            parser: TypedCommitParser::default(),
            formatter: DefaultChangelogFormatter::new(None, default_changelog_groups()),
            config,
            force: false,
            prerelease_id: None,
            draft: false,
        }
    }

    fn raw_commit(msg: &str) -> Commit {
        Commit {
            sha: "a".repeat(40),
            message: msg.into(),
        }
    }

    // --- plan() tests ---

    #[test]
    fn plan_no_commits_returns_error() {
        let s = make_strategy(vec![], vec![], Config::default());
        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoCommits { .. }));
    }

    #[test]
    fn plan_no_releasable_returns_error() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("chore: tidy up")],
            Config::default(),
        );
        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoBump { .. }));
    }

    #[test]
    fn force_releases_patch_when_no_releasable_commits() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "d".repeat(40),
        };
        let mut s = make_strategy(
            vec![tag],
            vec![raw_commit("chore: rename package")],
            Config::default(),
        );
        s.force = true;
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(1, 2, 4));
        assert_eq!(plan.bump, BumpLevel::Patch);
    }

    #[test]
    fn plan_first_release() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: initial feature")],
            Config::default(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(0, 1, 0));
        assert_eq!(plan.tag_name, "v0.1.0");
        assert!(plan.current_version.is_none());
    }

    #[test]
    fn plan_skips_commits_matching_skip_patterns() {
        let s = make_strategy(
            vec![],
            vec![
                raw_commit("feat: real feature"),
                raw_commit("feat: noisy experiment [skip release]"),
                raw_commit("fix: swallowed fix [skip sr]"),
            ],
            test_config(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.commits.len(), 1);
        assert_eq!(plan.commits[0].description, "real feature");
    }

    #[test]
    fn plan_custom_skip_patterns_override_defaults() {
        let git = GitConfig {
            skip_patterns: vec!["DO-NOT-RELEASE".into()],
            ..Default::default()
        };
        let s = make_strategy(
            vec![],
            vec![
                raw_commit("feat: shipped"),
                raw_commit("feat: DO-NOT-RELEASE internal"),
                // default patterns no longer active → this commit counts
                raw_commit("feat: still here [skip release]"),
            ],
            config_with_git(git),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.commits.len(), 2);
    }

    #[test]
    fn plan_increments_existing() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "b".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("fix: patch bug")],
            Config::default(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(1, 2, 4));
    }

    #[test]
    fn plan_breaking_bump() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "c".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("feat!: breaking change")],
            Config::default(),
        );
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(2, 0, 0));
    }

    #[test]
    fn plan_v0_breaking_downshifts_to_minor() {
        let tag = TagInfo {
            name: "v0.5.0".into(),
            version: Version::new(0, 5, 0),
            sha: "c".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("feat!: breaking change")],
            Config::default(),
        );
        let plan = s.plan().unwrap();
        // v0 protection: Major → Minor, so 0.5.0 → 0.6.0 (not 1.0.0)
        assert_eq!(plan.next_version, Version::new(0, 6, 0));
        assert_eq!(plan.bump, BumpLevel::Minor);
    }

    #[test]
    fn plan_v0_breaking_with_protection_disabled_bumps_major() {
        let tag = TagInfo {
            name: "v0.5.0".into(),
            version: Version::new(0, 5, 0),
            sha: "c".repeat(40),
        };
        let mut config = Config::default();
        config.git.v0_protection = false;
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("feat!: breaking change")],
            config,
        );
        let plan = s.plan().unwrap();
        // v0_protection: false allows bumping to v1
        assert_eq!(plan.next_version, Version::new(1, 0, 0));
        assert_eq!(plan.bump, BumpLevel::Major);
    }

    #[test]
    fn plan_v0_feat_stays_minor() {
        let tag = TagInfo {
            name: "v0.5.0".into(),
            version: Version::new(0, 5, 0),
            sha: "c".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("feat: new feature")],
            Config::default(),
        );
        let plan = s.plan().unwrap();
        // Non-breaking feat in v0 stays as minor bump
        assert_eq!(plan.next_version, Version::new(0, 6, 0));
        assert_eq!(plan.bump, BumpLevel::Minor);
    }

    #[test]
    fn plan_v0_fix_stays_patch() {
        let tag = TagInfo {
            name: "v0.5.0".into(),
            version: Version::new(0, 5, 0),
            sha: "c".repeat(40),
        };
        let s = make_strategy(
            vec![tag],
            vec![raw_commit("fix: bug fix")],
            Config::default(),
        );
        let plan = s.plan().unwrap();
        // Fix in v0 stays as patch
        assert_eq!(plan.next_version, Version::new(0, 5, 1));
        assert_eq!(plan.bump, BumpLevel::Patch);
    }

    // --- execute() tests ---

    #[test]
    fn execute_dry_run_no_side_effects() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();
        s.execute(&plan, true).unwrap();

        assert!(s.git.created_tags.lock().unwrap().is_empty());
        assert!(s.git.pushed_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_creates_and_pushes_tag() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        assert_eq!(*s.git.created_tags.lock().unwrap(), vec!["v0.1.0"]);
        assert_eq!(*s.git.pushed_tags.lock().unwrap(), vec!["v0.1.0"]);
    }

    #[test]
    fn execute_calls_vcs_create_release() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let releases = s.vcs.releases.lock().unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].0, "v0.1.0");
        assert!(!releases[0].1.is_empty());
    }

    #[test]
    fn execute_commits_changelog_before_tag() {
        let dir = tempfile::tempdir().unwrap();
        let changelog_path = dir.path().join("CHANGELOG.md");

        // Use the temp dir as the package path so auto-detection finds no version files.
        let config = Config {
            changelog: ChangelogConfig {
                file: Some(changelog_path.to_str().unwrap().to_string()),
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: dir.path().to_str().unwrap().to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        // Verify changelog was committed
        let committed = s.git.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(
            committed[0].0,
            vec![changelog_path.to_str().unwrap().to_string()]
        );
        assert!(committed[0].1.contains("chore(release): v0.1.0"));

        // Verify tag was created after commit
        assert_eq!(*s.git.created_tags.lock().unwrap(), vec!["v0.1.0"]);
    }

    #[test]
    fn execute_skips_existing_tag() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();

        // Pre-populate the tag to simulate it already existing
        s.git
            .created_tags
            .lock()
            .unwrap()
            .push("v0.1.0".to_string());

        s.execute(&plan, false).unwrap();

        // Tag should not be created again (still only the one we pre-populated)
        assert_eq!(s.git.created_tags.lock().unwrap().len(), 1);
    }

    #[test]
    fn execute_skips_existing_release() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();

        // Pre-populate a release to simulate it already existing
        s.vcs
            .releases
            .lock()
            .unwrap()
            .push(("v0.1.0".to_string(), "old notes".to_string()));

        s.execute(&plan, false).unwrap();

        // Should have updated in place without deleting
        let deleted = s.vcs.deleted_releases.lock().unwrap();
        assert!(deleted.is_empty(), "update should not delete");

        let releases = s.vcs.releases.lock().unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].0, "v0.1.0");
        assert_ne!(releases[0].1, "old notes");
    }

    #[test]
    fn execute_idempotent_rerun() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();

        // First run
        s.execute(&plan, false).unwrap();

        // Second run should also succeed (idempotent)
        s.execute(&plan, false).unwrap();

        // Tag should only have been created once (second run skips because tag_exists)
        assert_eq!(s.git.created_tags.lock().unwrap().len(), 1);

        // Tag push should only happen once (second run skips because remote_tag_exists)
        assert_eq!(s.git.pushed_tags.lock().unwrap().len(), 1);

        // Push (commit) should happen twice (always safe)
        assert_eq!(*s.git.push_count.lock().unwrap(), 2);

        // Release should be updated in place on second run (no delete)
        let deleted = s.vcs.deleted_releases.lock().unwrap();
        assert!(deleted.is_empty(), "update should not delete");

        let releases = s.vcs.releases.lock().unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].0, "v0.1.0");
    }

    #[test]
    fn execute_bumps_version_files() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_path,
            "[package]\nname = \"test\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();

        let config = Config {
            changelog: ChangelogConfig {
                file: None,
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: ".".into(),
                version_files: vec![cargo_path.to_str().unwrap().to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        // Verify the file was bumped
        let contents = std::fs::read_to_string(&cargo_path).unwrap();
        assert!(contents.contains("version = \"0.1.0\""));

        // Verify it was staged alongside the commit
        let committed = s.git.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert!(
            committed[0]
                .0
                .contains(&cargo_path.to_str().unwrap().to_string())
        );
    }

    #[test]
    fn execute_stages_changelog_and_version_files_together() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_path,
            "[package]\nname = \"test\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();

        let changelog_path = dir.path().join("CHANGELOG.md");

        let config = Config {
            changelog: ChangelogConfig {
                file: Some(changelog_path.to_str().unwrap().to_string()),
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: ".".into(),
                version_files: vec![cargo_path.to_str().unwrap().to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        // Both changelog and version file should be staged in a single commit
        let committed = s.git.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert!(
            committed[0]
                .0
                .contains(&changelog_path.to_str().unwrap().to_string())
        );
        assert!(
            committed[0]
                .0
                .contains(&cargo_path.to_str().unwrap().to_string())
        );
    }

    // --- artifact upload tests ---

    #[test]
    fn execute_uploads_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.tar.gz"), "fake tarball").unwrap();
        std::fs::write(dir.path().join("app.zip"), "fake zip").unwrap();

        let config = Config {
            changelog: ChangelogConfig {
                file: None,
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: ".".into(),
                version_files: vec!["__sr_test_dummy_no_bump__".into()],
                artifacts: vec![
                    dir.path().join("*.tar.gz").to_str().unwrap().to_string(),
                    dir.path().join("*.zip").to_str().unwrap().to_string(),
                ],
                ..Default::default()
            }],
            ..Default::default()
        };

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let uploaded = s.vcs.uploaded_assets.lock().unwrap();
        assert_eq!(uploaded.len(), 1);
        assert_eq!(uploaded[0].0, "v0.1.0");
        assert_eq!(uploaded[0].1.len(), 2);
        assert!(uploaded[0].1.iter().any(|f| f.ends_with("app.tar.gz")));
        assert!(uploaded[0].1.iter().any(|f| f.ends_with("app.zip")));
    }

    #[test]
    fn execute_dry_run_shows_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.tar.gz"), "fake tarball").unwrap();

        let config = Config {
            changelog: ChangelogConfig {
                file: None,
                ..Default::default()
            },
            packages: vec![PackageConfig {
                path: ".".into(),
                version_files: vec!["__sr_test_dummy_no_bump__".into()],
                artifacts: vec![dir.path().join("*.tar.gz").to_str().unwrap().to_string()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        s.execute(&plan, true).unwrap();

        // No uploads should happen during dry-run
        let uploaded = s.vcs.uploaded_assets.lock().unwrap();
        assert!(uploaded.is_empty());
    }

    #[test]
    fn execute_no_artifacts_skips_upload() {
        let s = make_strategy(vec![], vec![raw_commit("feat: something")], test_config());
        let plan = s.plan().unwrap();
        s.execute(&plan, false).unwrap();

        let uploaded = s.vcs.uploaded_assets.lock().unwrap();
        assert!(uploaded.is_empty());
    }

    #[test]
    fn resolve_globs_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let pattern = dir.path().join("*.txt").to_str().unwrap().to_string();
        let result = resolve_globs(&[pattern]).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|f: &String| f.ends_with("a.txt")));
        assert!(result.iter().any(|f: &String| f.ends_with("b.txt")));
    }

    #[test]
    fn resolve_globs_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.txt"), "data").unwrap();

        let pattern = dir.path().join("*.txt").to_str().unwrap().to_string();
        // Same pattern twice should not produce duplicates
        let result = resolve_globs(&[pattern.clone(), pattern]).unwrap();
        assert_eq!(result.len(), 1);
    }

    // --- floating tags tests ---

    #[test]
    fn plan_floating_tag_when_enabled() {
        let tag = TagInfo {
            name: "v3.2.0".into(),
            version: Version::new(3, 2, 0),
            sha: "d".repeat(40),
        };
        let config = config_with_git(GitConfig {
            floating_tag: true,
            ..Default::default()
        });

        let s = make_strategy(vec![tag], vec![raw_commit("fix: patch")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(3, 2, 1));
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v3"));
    }

    #[test]
    fn plan_no_floating_tag_when_disabled() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            config_with_git(GitConfig {
                floating_tag: false,
                ..Default::default()
            }),
        );
        let plan = s.plan().unwrap();
        assert!(plan.floating_tag_name.is_none());
    }

    #[test]
    fn plan_floating_tag_custom_prefix() {
        let tag = TagInfo {
            name: "release-2.5.0".into(),
            version: Version::new(2, 5, 0),
            sha: "e".repeat(40),
        };
        let config = config_with_git(GitConfig {
            floating_tag: true,
            tag_prefix: "release-".into(),
            ..Default::default()
        });

        let s = make_strategy(vec![tag], vec![raw_commit("fix: patch")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("release-2"));
    }

    #[test]
    fn execute_floating_tags_force_create_and_push() {
        let config = config_with_git(GitConfig {
            floating_tag: true,
            ..Default::default()
        });

        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "f".repeat(40),
        };
        let s = make_strategy(vec![tag], vec![raw_commit("fix: a bug")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v1"));

        s.execute(&plan, false).unwrap();

        assert_eq!(*s.git.force_created_tags.lock().unwrap(), vec!["v1"]);
        assert_eq!(*s.git.force_pushed_tags.lock().unwrap(), vec!["v1"]);
    }

    #[test]
    fn execute_no_floating_tags_when_disabled() {
        let s = make_strategy(
            vec![],
            vec![raw_commit("feat: something")],
            config_with_git(GitConfig {
                floating_tag: false,
                ..Default::default()
            }),
        );
        let plan = s.plan().unwrap();
        assert!(plan.floating_tag_name.is_none());

        s.execute(&plan, false).unwrap();

        assert!(s.git.force_created_tags.lock().unwrap().is_empty());
        assert!(s.git.force_pushed_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_floating_tags_dry_run_no_side_effects() {
        let config = config_with_git(GitConfig {
            floating_tag: true,
            ..Default::default()
        });

        let tag = TagInfo {
            name: "v2.0.0".into(),
            version: Version::new(2, 0, 0),
            sha: "a".repeat(40),
        };
        let s = make_strategy(vec![tag], vec![raw_commit("fix: something")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v2"));

        s.execute(&plan, true).unwrap();

        assert!(s.git.force_created_tags.lock().unwrap().is_empty());
        assert!(s.git.force_pushed_tags.lock().unwrap().is_empty());
    }

    #[test]
    fn execute_floating_tags_idempotent() {
        let config = config_with_git(GitConfig {
            floating_tag: true,
            ..Default::default()
        });

        let s = make_strategy(vec![], vec![raw_commit("feat: something")], config);
        let plan = s.plan().unwrap();
        assert_eq!(plan.floating_tag_name.as_deref(), Some("v0"));

        // Run twice
        s.execute(&plan, false).unwrap();
        s.execute(&plan, false).unwrap();

        // Force ops run every time (correct for floating tags)
        assert_eq!(s.git.force_created_tags.lock().unwrap().len(), 2);
        assert_eq!(s.git.force_pushed_tags.lock().unwrap().len(), 2);
    }

    // --- force mode tests ---

    #[test]
    fn force_rerelease_when_tag_at_head() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "a".repeat(40),
        };
        let mut s = make_strategy(vec![tag], vec![], Config::default());
        // HEAD == tag SHA, and no new commits
        s.git.head = "a".repeat(40);
        s.force = true;

        let plan = s.plan().unwrap();
        assert_eq!(plan.next_version, Version::new(1, 2, 3));
        assert_eq!(plan.tag_name, "v1.2.3");
        assert!(plan.commits.is_empty());
        assert_eq!(plan.current_version, Some(Version::new(1, 2, 3)));
    }

    #[test]
    fn force_fails_when_tag_not_at_head() {
        let tag = TagInfo {
            name: "v1.2.3".into(),
            version: Version::new(1, 2, 3),
            sha: "a".repeat(40),
        };
        let mut s = make_strategy(vec![tag], vec![], Config::default());
        // HEAD != tag SHA
        s.git.head = "b".repeat(40);
        s.force = true;

        let err = s.plan().unwrap_err();
        assert!(matches!(err, ReleaseError::NoCommits { .. }));
    }
}
