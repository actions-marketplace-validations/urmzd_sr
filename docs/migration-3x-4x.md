# Migrating from sr 3.x to 4.x

sr 4.0 restructures the CLI around a clean trunk-based workflow pipeline: `worktree` → `commit` → `pr` → `review` → `release`. Several commands were removed, folded, or redesigned.

## Breaking changes

### Commands removed

| 3.x command | What to do |
|-------------|------------|
| `sr ask` | Removed. Use Claude Code, Copilot Chat, or any AI assistant for freeform questions. |
| `sr explain` | Removed. Use `git show <rev>` with your AI assistant. |

### Commands folded

| 3.x command | 4.x equivalent |
|-------------|----------------|
| `sr rebase` | `sr commit --rebase` (add `--last N` for specific count) |
| `sr plan` | `sr status` (shows version, commits, and PRs automatically) |
| `sr version` | `sr status` (version shown automatically) or `sr status --format json \| jq .next_version` |
| `sr changelog` | Removed as standalone. Changelog is written as part of `sr release`. |
| `sr changelog --write` | Part of `sr release` now. |

### Commands redesigned

#### `sr review`

**3.x:** Reviewed local diffs (staged changes or base ref).

```bash
# 3.x
sr review                    # review all local changes
sr review --staged           # review staged only
sr review --base main        # review against base ref
```

**4.x:** Reviews the GitHub PR for the current branch. Requires `GH_TOKEN` or `GITHUB_TOKEN`.

```bash
# 4.x
sr review                    # review current branch's PR
sr review -M "focus on auth" # with context
sr review --comment          # post review as GitHub comment
```

The old local-diff review behavior is no longer available. Use your AI assistant for local reviews.

#### `sr branch` → `sr worktree`

**3.x:** Suggested a branch name and optionally created it via `git checkout -b`.

```bash
# 3.x
sr branch "add user auth"   # suggest name from description
sr branch --create           # suggest and create
```

**4.x:** Replaced by `sr worktree` — creates a git worktree + branch, moving uncommitted changes off trunk. This keeps `main` clean and uses worktrees as the primitive for parallel work.

```bash
# 4.x
sr worktree -M "add user auth"  # creates worktree at ../repo-feat/add-user-auth/
sr worktree                      # names branch from current changes
```

#### `sr pr`

**3.x:** Generated PR content and optionally created it.

```bash
# 3.x
sr pr --create               # generate and create
sr pr --create --draft       # create as draft
sr pr --base develop         # target specific base
```

**4.x:** Always creates/updates the PR. Base branch auto-detected from `sr.yaml` `branches` config.

```bash
# 4.x
sr pr                        # generate and create
sr pr --draft                # create as draft
sr pr -M "focus on the API"  # with context
```

### Flags removed

| 3.x flag | 4.x status |
|----------|------------|
| `sr commit --staged` | Removed. AI always sees all changes and groups them into logical commits. |
| `sr branch --create` | Use `sr worktree`. |
| `sr branch <description>` | Use `sr worktree -M "description"`. |
| `sr pr --create` | Removed. PR is always created/updated. |
| `sr pr --base <branch>` | Removed. Auto-detected from `sr.yaml` `branches` config (first entry). |
| `sr review --staged` | Removed. Review now targets GitHub PRs. |
| `sr review --base <ref>` | Removed. Review now targets GitHub PRs. |

### Flags added

| Flag | Command | Description |
|------|---------|-------------|
| `-M, --message` | `worktree`, `pr`, `review` | Context/instructions for AI (already existed on `commit`) |
| `--rebase` | `commit` | Switch to rebase mode (was `sr rebase`) |
| `--last N` | `commit` | Number of commits for rebase (requires `--rebase`) |
| `--comment` | `review` | Post review as GitHub PR comment |
| `-c, --channel` | `release` | Named release channel from `sr.yaml` |

## New features

### Release channels

Define named release channels in `sr.yaml` for multi-environment workflows:

```yaml
channels:
  canary:
    prerelease: canary
    branches: [main]
  rc:
    prerelease: rc
    branches: [release/*]
  stable:
    branches: [main]

default_channel: stable
```

Use with: `sr release --channel canary`

Each channel can override `prerelease`, `branches`, `draft`, and `artifacts` from the root config. When `default_channel` is set, it applies automatically unless overridden by `--channel`.

### `sr status` (replaces `sr plan`, `sr version`, `sr changelog`)

Single command shows everything:

```bash
$ sr status
  Branch: main
  Current: v1.3.0
  Next: v1.4.0 (minor)
  Commits: 7
    feat(auth): add OAuth support
    fix(api): null check on response
    ...
  Open PRs: 3 (2 ready, 1 draft)

$ sr status --format json   # machine-readable, includes changelog preview
```

Replaces `sr plan`, `sr version`, and `sr changelog` (preview). Changelog writing is now part of `sr release`.

## Config restructured

The flat `sr.yaml` is now grouped by concern:

```yaml
# 3.x (flat)
branches: [main]
tag_prefix: "v"
commit_pattern: '...'
types: [...]
build_command: "..."
pre_release_command: "..."
hooks:
  commit-msg: [sr hook commit-msg]

# 4.x (grouped by operation)
commit:
  pattern: '...'
  types: [...]

release:
  branches: [main]
  tag_prefix: "v"
  version_files: [Cargo.toml]
  channels:
    canary: { prerelease: canary }

hooks:
  pre_commit: ["cargo fmt --check"]
  pre_release: ["cargo test"]
  post_release: ["./notify.sh"]
```

**Removed fields:** `build_command`, `pre_release_command`, `post_release_command`, `lifecycle` — use `hooks.pre_release` / `hooks.post_release` instead.

**Renamed:** `commit_pattern` → `commit.pattern`, `breaking_section` → `commit.breaking_section`.

**Hooks restructured:** Git hook names (`commit-msg`, `pre-commit`) replaced with lifecycle events (`pre_commit`, `pre_release`, etc.). Structured steps with patterns removed — use simple shell commands.

## Architecture changes

Consolidated from 5 crates to 2. All logic lives in `sr-core`; `sr-cli` is the thin CLI binary.

| Crate | 3.x | 4.x |
|-------|-----|-----|
| `sr-ai` | AI backends, commands, UI | Removed (merged into sr-core) |
| `sr-git` | Git implementation | Removed (merged into sr-core) |
| `sr-github` | GitHub API | Removed (merged into sr-core) |
| `sr-core` | Release logic only | Everything: AI, git, GitHub, config, release |
| `sr-cli` | Thin dispatch + some logic | CLI only: commands, UI, args |

### For library consumers

All imports now come from `sr_core`:

```rust
// 3.x
use sr_ai::commands::commit::{CommitPlan, CommitArgs};
use sr_core::config::ReleaseConfig;
let config = ReleaseConfig::load(path)?;

// 4.x
use sr_core::ai::services::commit::{CommitPlan, PlanInput, generate_plan, execute_plan};
use sr_core::config::Config;
let config = Config::load(path)?;
// Fields are now nested: config.commit.types, config.release.tag_prefix, etc.
```

## CI/CD migration

### GitHub Action inputs

The `command` input still accepts all valid commands, but removed commands will error:

```yaml
# 3.x — these no longer work
- uses: urmzd/sr@v4
  with:
    command: ask    # error: removed
    command: explain # error: removed

# 4.x — use updated command names
- uses: urmzd/sr@v4
  with:
    command: plan   # works (subcommands via additional args)
```

### Scripts referencing `sr version` or `sr changelog`

```bash
# 3.x
VERSION=$(sr version --short)
sr changelog --write
sr plan --format json

# 4.x
VERSION=$(sr status --format json | jq -r '.next_version')
# Changelog is now written automatically by sr release
sr status --format json
```
