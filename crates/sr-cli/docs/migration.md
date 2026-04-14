# Migration Guide

## Overview

| Version | Theme | Key changes |
|---------|-------|-------------|
| **3.x** | AI-powered CLI | Built-in AI backends, git hooks, flat config |
| **4.x** | Config restructure + MCP | Nested config, MCP server, AI backends removed (commands kept as thin wrappers) |
| **5.x** | Release-only CLI | CLI commands stripped to release engineering; all git/PR/review workflows move to MCP tools |
| **6.x** | MCP-first workflows | PR, worktree, and breaking-commit tools added to MCP server; `sr init` improved |

---

## Migrating from 3.x to 4.x

### Config restructured

The flat `sr.yaml` was reorganized into three sections — `commit`, `release`, `hooks`:

```yaml
# 3.x (flat)
branches: [main]
tag_prefix: "v"
commit_pattern: '...'
types: [...]
build_command: "cargo build --release"
pre_release_command: "cargo test"
hooks:
  commit-msg: [sr hook commit-msg]

# 4.x (grouped by concern)
commit:
  pattern: '...'
  types: [...]

release:
  branches: [main]
  tag_prefix: "v"
  version_files: [Cargo.toml]
  floating_tags: true

hooks:
  pre_release: ["cargo test"]
  post_release: ["./notify.sh"]
```

Field mappings:

| 3.x field | 4.x location |
|-----------|--------------|
| `branches` | `release.branches` |
| `tag_prefix` | `release.tag_prefix` |
| `commit_pattern` | `commit.pattern` |
| `breaking_section` | `commit.breaking_section` |
| `misc_section` | `commit.misc_section` |
| `types` | `commit.types` |
| `version_files` | `release.version_files` |
| `floating_tags` | `release.floating_tags` |
| `artifacts` | `release.artifacts` |
| `stage_files` | `release.stage_files` |
| `build_command` | Removed — use `hooks.pre_release` |
| `pre_release_command` | `hooks.pre_release` |
| `post_release_command` | `hooks.post_release` |
| `hooks.commit-msg` | Removed |
| `hooks.pre-commit` | Removed |

### AI backends removed

sr 3.x called AI providers directly. In 4.x, the AI module was removed. Commands
(`commit`, `pr`, `review`, `worktree`, `rebase`) were kept as thin non-AI wrappers
that accept explicit flags instead of generating content.

### MCP server added

`sr mcp serve` exposes git operations as tools for AI assistants (Claude Code,
Gemini CLI, etc.). `sr init` creates both `sr.yaml` and `.mcp.json`.

### Git hooks removed

sr 3.x installed git hooks (`commit-msg`, `pre-commit`) via `.githooks/`. These
no longer exist. Remove them:

```bash
rm -rf .githooks/
rm -f .git/hooks/commit-msg .git/hooks/pre-commit .git/hooks/pre-push
git config --unset core.hooksPath 2>/dev/null || true
```

The 4.x `hooks:` section uses lifecycle event names (`pre_release`, `post_release`)
instead of git hook names.

### Crate consolidation

5 crates consolidated to 2:

| 3.x crate | 4.x status |
|------------|------------|
| `sr-ai` | Removed |
| `sr-git` | Merged into `sr-core` |
| `sr-github` | Merged into `sr-core` |
| `sr-core` | All release logic, git, GitHub, config |
| `sr-cli` | CLI dispatch only |

### GitHub Action: v3 → v4

Update `@v3` → `@v4`:

```yaml
# Before
- uses: urmzd/sr@v3
# After
- uses: urmzd/sr@v4
```

**Inputs removed in v4:**

| v3 input | What happened |
|----------|---------------|
| `command` | Removed. v3 accepted any subcommand (`release`, `plan`, `commit`, `pr`, etc.). v4 always runs `sr release` or `sr status`. |
| `force` | Removed in v4 (restored in v5). |
| `config` | Removed. sr always reads `sr.yaml` from the repo root. |
| `artifacts` | Removed in v4 (restored in v5). v3 accepted newline/comma-separated globs. |
| `build-command` | Removed. Use `hooks.pre_release` in `sr.yaml` instead. |

**Execution model changes:**

| Aspect | v3 | v4 |
|--------|----|----|
| Command dispatch | Configurable — any sr subcommand via `command` input | Fixed — always `sr release` or `sr status --format json` |
| Arg passing | Array-based (`sr "${ARGS[@]}"`) | Simple string (`sr $CMD`) |
| Artifact handling | Parsed from newline/comma input, each passed as `--artifacts` | Not supported |
| Build command | Written to temp script, passed as `--build-command` | Removed |
| Logging | `::group::` blocks with verbose echo | Minimal, no grouping |
| Exit code 2 | Sets all outputs to empty strings | Only sets `released=false` |

---

## Migrating from 4.x to 5.x

### CLI commands removed

The thin wrapper commands added in 4.x were removed. These workflows now live
entirely in the MCP server tools:

| Removed CLI command | MCP tool equivalent |
|---------------------|---------------------|
| `sr commit` | `sr_commit` (via `sr mcp serve`) |
| `sr pr` | `sr_pr_template` + `sr_pr_create` |
| `sr review` | AI assistant + `gh` CLI |
| `sr worktree` | `sr_worktree` + `sr_worktree_list` + `sr_worktree_remove` |
| `sr rebase` | `git rebase` directly |
| `sr plan` | `sr status` |
| `sr version` | `sr status --format json \| jq .next_version` |
| `sr changelog` | Generated automatically by `sr release` |

**Remaining CLI commands:** `release`, `status`, `config`, `init`, `mcp`, `migrate`,
`completions`, `update`.

### MCP tools expanded

New tools available via `sr mcp serve`:

| Tool | Description |
|------|-------------|
| `sr_commit` | Now supports `breaking` flag — auto-adds `!` and `BREAKING CHANGE:` footer |
| `sr_pr_template` | Generates PR template from branch commits/diff stats |
| `sr_pr_create` | Creates GitHub PR via `gh` CLI (title, body, labels, draft) |
| `sr_worktree` | Creates worktrees under `.sr/worktrees/` with metadata tracking |
| `sr_worktree_list` | Lists all worktrees with descriptions and creation dates |
| `sr_worktree_remove` | Removes worktree and cleans up metadata |

### Dirty working tree check removed

sr 4.x refused to release if the working tree had uncommitted changes. This check
was removed in 5.x because sr only stages files it explicitly modifies (version
files, changelog, stage_files) — it never runs `git add -A`. Unrelated files in
the working tree (downloaded CI artifacts, build outputs) are harmless and were
causing false failures.

### GitHub Action: v4 → v5

Update `@v4` → `@v5`:

```yaml
# Before
- uses: urmzd/sr@v4

# After — basic
- uses: urmzd/sr@v5

# After — with artifacts
- uses: urmzd/sr@v5
  with:
    github-token: ${{ steps.app-token.outputs.token }}
    artifacts: "release-assets/*"
```

**Inputs restored from v3:**

| Input | v3 | v4 | v5 |
|-------|:---:|:---:|:---:|
| `force` | `false` | Removed | `false` — adds `--force` |
| `artifacts` | Newline/comma-separated | Removed | Space-separated, each passed as `--artifacts "glob"` |

**New inputs in v5:**

| Input | Default | Description |
|-------|---------|-------------|
| `package` | `""` | Target a specific monorepo package |
| `channel` | `""` | Release channel (e.g. canary, rc, stable) |
| `prerelease` | `""` | Pre-release identifier (e.g. alpha, beta, rc) |
| `stage-files` | `""` | Additional file globs to stage in release commit (space-separated) |
| `sign-tags` | `false` | Sign tags with GPG/SSH |
| `draft` | `false` | Create GitHub release as a draft |

**Execution model changes:**

| Aspect | v4 | v5 |
|--------|----|----|
| Arg passing | Simple string (`sr $CMD`) | `eval sr $CMD` (supports quoted globs) |
| Dirty tree check | sr refuses if working tree is dirty | Removed — unrelated files are harmless |
| CLI flag coverage | Only `--dry-run` | All `sr release` flags exposed as inputs |

**Inputs unchanged across all versions:**

| Input | Default |
|-------|---------|
| `dry-run` | `false` |
| `github-token` | `${{ github.token }}` |
| `git-user-name` | `sr[bot]` |
| `git-user-email` | `sr[bot]@urmzd.com` |
| `sha256` | `""` |

**Outputs unchanged across all versions:**

| Output | Description |
|--------|-------------|
| `version` | Released version |
| `previous-version` | Previous version |
| `tag` | Git tag created |
| `bump` | Bump level (major/minor/patch) |
| `floating-tag` | Floating major tag (e.g. `v3`) |
| `commit-count` | Commits included |
| `released` | `true`/`false` |
| `json` | Full release metadata as JSON |

### `sr init` improvements

- No longer fails if `sr.yaml` already exists — each step (sr.yaml, .mcp.json,
  .gitignore) runs independently
- Automatically adds `.sr/` to `.gitignore` (used for worktree metadata and cache)

### Worktree management via `.sr/`

Worktrees created via MCP (`sr_worktree`) are stored under `.sr/worktrees/`
instead of sibling directories. Each worktree gets a metadata file at
`.sr/worktrees/<branch>.json` tracking its purpose, description, and creation
date. The `.sr/` directory is automatically gitignored by `sr init`.

---

## Migrating from 5.x to 6.x

v6 is a non-breaking release that was triggered by a version bump. There are no
breaking changes for users. All v5 config, action inputs, and CLI commands work
unchanged.

### What's new in v6

**MCP tools added:**

| Tool | Description |
|------|-------------|
| `sr_commit` | Now supports `breaking` flag — auto-adds `!` suffix and `BREAKING CHANGE:` footer |
| `sr_pr_template` | Generates PR template from branch commits and diff stats |
| `sr_pr_create` | Creates GitHub PR via `gh` CLI (title, body, labels, draft) |
| `sr_worktree` | Creates worktrees under `.sr/worktrees/` with metadata tracking |
| `sr_worktree_list` | Lists all worktrees with descriptions and creation dates |
| `sr_worktree_remove` | Removes worktree and cleans up metadata |

**`sr init` improvements:**

- No longer fails if `sr.yaml` already exists — each step (sr.yaml, .mcp.json,
  .gitignore) runs independently and skips files that already exist
- Automatically adds `.sr/` to `.gitignore` (used for worktree metadata)

### GitHub Action: v5 → v6

Update `@v5` → `@v6`. All inputs and outputs are unchanged:

```yaml
# Before
- uses: urmzd/sr@v5

# After
- uses: urmzd/sr@v6
```

### Worktree management via `.sr/`

Worktrees created via MCP (`sr_worktree`) are stored under `.sr/worktrees/`
instead of sibling directories. Each worktree gets a metadata file at
`.sr/worktrees/<branch>.json` tracking its purpose, description, and creation
date. The `.sr/` directory is automatically gitignored by `sr init`.

---

## Migrating from 3.x directly to 6.x

Follow the 3.x→4.x, 4.x→5.x, and 5.x→6.x sections above in order, or use
this quick checklist:

1. **Remove git hooks** — delete `.githooks/`, unset `core.hooksPath`
2. **Restructure sr.yaml** — move flat fields into `commit:`, `release:`, `hooks:` sections
3. **Update action** — change `@v3` to `@v6`
4. **Update scripts** — replace `sr version`, `sr changelog`, `sr plan` with `sr status`

### Action input migration: v3 → v6

| v3 input | v6 equivalent |
|----------|---------------|
| `command: release` | Default (no input needed) |
| `command: plan` | `dry-run: "true"` |
| `command: <other>` | Removed — use CLI or MCP tools |
| `artifacts: "dist/*\nbin/*"` | `artifacts: "dist/* bin/*"` (space-separated) |
| `build-command: "make"` | Removed — use `hooks.pre_release` in sr.yaml |
| `config: custom.yaml` | Removed — always reads `sr.yaml` |
| `force: true` | `force: "true"` (same) |

### CI/CD script migration

```bash
# 3.x
VERSION=$(sr version --short)
sr changelog --write
sr plan --format json

# 6.x
VERSION=$(sr status --format json | jq -r '.next_version')
# Changelog is written automatically by sr release
sr status --format json
```
