# Changelog

## 3.3.7 (2026-04-09)

### Bug Fixes

- **ci**: remove --allow-dirty from cargo publish ([c9fb557](https://github.com/urmzd/sr/commit/c9fb5576d942d0fa2b1c191fd20187a603d2c493))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.6...v3.3.7)


## 3.3.6 (2026-04-09)

### Documentation

- add LICENSE to sub-crates for publishing compliance ([47ad907](https://github.com/urmzd/sr/commit/47ad90737985d3432d9bb8972c1afa067cdbdf8d))

### Refactoring

- **update**: use agentspec update mechanism ([359fae6](https://github.com/urmzd/sr/commit/359fae6b635f48f4db0a298cb0f2ff01f8e9a6b6))
- **ui**: extract UI primitives to agentspec-ui library ([60cd339](https://github.com/urmzd/sr/commit/60cd3398d4ba48651a080fba7d4dd8d31c9c9194))
- **ai**: migrate to agentspec-provider library ([a582529](https://github.com/urmzd/sr/commit/a58252930c96dcb57bd4e78df93d44a42bdf79d1))

### Miscellaneous

- fix cargo fmt import formatting ([b8d15a5](https://github.com/urmzd/sr/commit/b8d15a54f422f55599fbd7324919c9fe338ecc36))
- **deps**: migrate agentspec crates to registry ([bb50305](https://github.com/urmzd/sr/commit/bb50305437a43a8d753012bb79631ff3da1139cf))
- **workflows**: remove agentspec repository checkout ([0582bfa](https://github.com/urmzd/sr/commit/0582bfae6aebfd2f5a31f20bfbe21b90657e18af))
- **workflows**: add agentspec checkout and workspace config ([b8615f0](https://github.com/urmzd/sr/commit/b8615f09783c205373c267864851ebb248d7218f))
- **gitignore**: ignore .fastembed_cache ([039fc00](https://github.com/urmzd/sr/commit/039fc00ed0759eee6ce7d5af02c59776a66c65b4))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.5...v3.3.6)


## 3.3.5 (2026-04-06)

### Bug Fixes

- **action**: hardcode public GitHub URLs for binary download ([e3437db](https://github.com/urmzd/sr/commit/e3437db1df205d9709997754fbbf7524f46c442b))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.4...v3.3.5)


## 3.3.4 (2026-04-06)

### Refactoring

- simplify release tag resolution in action ([28c94ca](https://github.com/urmzd/sr/commit/28c94ca5e5fdccb4e25e380b49b568467423cebd))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.3...v3.3.4)


## 3.3.3 (2026-04-06)

### Bug Fixes

- **sr-ai**: unquote C-quoted paths from git status --porcelain ([558b467](https://github.com/urmzd/sr/commit/558b46768fb540750b3bf295612583638a1bdbfa))
- **tests**: prevent changelog file pollution from release tests ([2950f9c](https://github.com/urmzd/sr/commit/2950f9cf6e9cca62bb0cc9af60a03a2d29d702d4))

### Documentation

- **sr-core**: update changelog with v0.1.0 entries ([a875757](https://github.com/urmzd/sr/commit/a8757573ace5a81bc3d6a95b13782671c060261c))

### Miscellaneous

- apply cargo fmt formatting ([c985790](https://github.com/urmzd/sr/commit/c985790555ec5d924cde0d02e808d05163ec659c))
- update Cargo.lock for v3.3.2 ([e3d87e0](https://github.com/urmzd/sr/commit/e3d87e03e84ac0edd59e0b65da1c7df989536809))
- add linguist overrides to fix language stats (#17) ([f40b27e](https://github.com/urmzd/sr/commit/f40b27ec9a0915c4e6e259bfbf478216e1c528e5))
- **deps**: bump actions/download-artifact from 4 to 8 ([c89b261](https://github.com/urmzd/sr/commit/c89b2618d999024a1935832621e0b08149940918))
- **deps**: bump actions/create-github-app-token from 1 to 3 ([445b3e8](https://github.com/urmzd/sr/commit/445b3e83b9ef1b71d252cc675fc6aa3d4eef50ae))
- **deps**: bump actions/upload-artifact from 4 to 7 ([09be4c2](https://github.com/urmzd/sr/commit/09be4c23f6f156af1fdf049616596aad7449426f))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.2...v3.3.3)


## 3.3.2 (2026-04-03)

### Bug Fixes

- **action**: remove unnecessary GitHub token authentication (#15) ([502b4fe](https://github.com/urmzd/sr/commit/502b4fe8c434ab86b987115f8814f7cac940a472))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.1...v3.3.2)


## 3.3.1 (2026-04-03)

### Bug Fixes

- **action**: always download sr binary from public GitHub (#14) ([faa54bb](https://github.com/urmzd/sr/commit/faa54bb658999ad5d6397ab76fa4a047c524c856))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.3.0...v3.3.1)


## 3.3.0 (2026-04-02)

### Features

- **action,install**: add GitHub Enterprise support with optional authentication ([930a6d7](https://github.com/urmzd/sr/commit/930a6d7cf9d7e52627ea57aa87b097666549999c))
- **commit**: make refactor commits trigger patch bump ([2c8b5de](https://github.com/urmzd/sr/commit/2c8b5de6743d4a32edbe623649abb8972c08404e))
- **config**: enable floating tags and update defaults ([6eee880](https://github.com/urmzd/sr/commit/6eee880ce79364460293db0567213f2cc0d999e1))

### Documentation

- **sr-core**: remove duplicate changelog entries ([39e9879](https://github.com/urmzd/sr/commit/39e9879ee2c0b25dcb1a0e90c9f918a332d8ae12))
- **changelog**: add changelog for version 3.2.4 ([defdafc](https://github.com/urmzd/sr/commit/defdafc735e3248148b057edabfd5aa5e04b1de9))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.2.5...v3.3.0)


## 3.2.5 (2026-04-01)

### Bug Fixes

- **action**: handle floating tag resolution under pipefail ([b0d6461](https://github.com/urmzd/sr/commit/b0d646191c630cf10c88e70f3a6626e81c16b628))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.2.4...v3.2.5)


## 3.2.4 (2026-04-01)

### Bug Fixes

- **action**: remove auth from release API calls for cross-repo compatibility ([8705e59](https://github.com/urmzd/sr/commit/8705e591c9a11c4f17c9b06c973f0164dcdd8289))

### Miscellaneous

- add diagnostic logging to action.yml ([464c0db](https://github.com/urmzd/sr/commit/464c0dbbcadfa75851293f579ae9c10daa94c334))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.2.3...v3.2.4)


## 3.2.3 (2026-04-01)

### Refactoring

- normalize action.yml metadata and remove redundant required fields ([8791859](https://github.com/urmzd/sr/commit/87918595fddac4f749ae4559fb59a3a24bc8a8fd))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.2.2...v3.2.3)


## 3.2.2 (2026-04-01)

### Refactoring

- extract sr binary download into standalone script with curl fallback (#10) ([fa1b52f](https://github.com/urmzd/sr/commit/fa1b52f2735d108d8bfd6a4ef55cc9be18003c58))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.2.1...v3.2.2)


## 3.2.1 (2026-04-01)

### Bug Fixes

- **sr-ai**: embed system prompt in user message and replace deprecated tools allowlist with TOML policy ([f685934](https://github.com/urmzd/sr/commit/f685934b4e6387f0e3e709560519792d7056428d))

### Refactoring

- **sr-ai**: move tempfile to runtime dependency ([bf28ad8](https://github.com/urmzd/sr/commit/bf28ad845d51f069343b9d350d87ea5e6f3eafb1))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.2.0...v3.2.1)


## 3.2.0 (2026-04-01)

### Features

- **install**: add optional SHA256 verification support ([027e01b](https://github.com/urmzd/sr/commit/027e01b31d9869b14d585c48fbf0c55e4447dffd))
- **action**: add optional SHA256 verification input ([946a3a4](https://github.com/urmzd/sr/commit/946a3a4cb0c7c6a3b835654e518752bd495d5195))

### Documentation

- update for checksum verification changes ([cc2465b](https://github.com/urmzd/sr/commit/cc2465be1f9c631dbcac9936b200e356969ebc17))

### Refactoring

- **release**: remove floating release and checksum features ([35ba5ea](https://github.com/urmzd/sr/commit/35ba5ea6f4c0821557ffd685c5f85165f707925e))
- **core**: remove sha2 dependency ([ede06bf](https://github.com/urmzd/sr/commit/ede06bfd0b859d45d8e28d4d501db2e1dfb0964f))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.1.0...v3.2.0)


## 3.1.0 (2026-03-30)

### Features

- verify sha256 checksum after binary download ([b940526](https://github.com/urmzd/sr/commit/b9405262ed4777a4e2ba0273436fb966775661f3))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.0.2...v3.1.0)


## 3.0.2 (2026-03-30)

### Bug Fixes

- pass action context through env for composite action compatibility ([c21ddd3](https://github.com/urmzd/sr/commit/c21ddd312d3b656c476a3c294221c0e8fcc39029))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.0.1...v3.0.2)


## 3.0.1 (2026-03-30)

### Bug Fixes

- swap floating release assets per-asset to minimise 404 window ([0be8abc](https://github.com/urmzd/sr/commit/0be8abc0f3678a003bd818fb32f04face9fffd9b))
- use action ref directly for binary download URL ([622b693](https://github.com/urmzd/sr/commit/622b69398faf49b727ec202207891c674b18103f))

### Documentation

- update sr action references from v2 to v3 ([ac4a5a9](https://github.com/urmzd/sr/commit/ac4a5a901c66de8d08b2d4d57a507236638f1b0e))

[Full Changelog](https://github.com/urmzd/sr/compare/v3.0.0...v3.0.1)


## 3.0.0 (2026-03-30)

### Breaking Changes

- **ai**: sandbox agent backends to read-only git access with working tree snapshots ([77a94df](https://github.com/urmzd/sr/commit/77a94df8b0ac9e13453ab36043311936d57d3281))
- remove lifecycle hooks, output structured JSON from sr release ([b93231b](https://github.com/urmzd/sr/commit/b93231b4e60946e82e5759a34956adc14392f8cd))

### Features

- **commit**: validate messages before execution with error recovery ([4ecba17](https://github.com/urmzd/sr/commit/4ecba1730fe1900c11e0e6ac820aa95b07b754f3))
- **ui**: add commit validation and failure display functions ([ae9b65d](https://github.com/urmzd/sr/commit/ae9b65dc28d6ca2cf4f62ba18eb741f18aaf1fa1))
- **commit**: add commit reorganization functionality ([7d504d2](https://github.com/urmzd/sr/commit/7d504d2d7b40a591c553c3e1a03619dbe8c8d1c6))
- **git**: add commits_since_last_tag method ([392029c](https://github.com/urmzd/sr/commit/392029ce079648046fcfec3400471d07a01a05b8))
- **cli**: implement automatic hook synchronization ([860d095](https://github.com/urmzd/sr/commit/860d095700ac9b5213473075be50492452d282ab))
- **cli**: implement --merge flag for incremental config updates ([f0226f1](https://github.com/urmzd/sr/commit/f0226f150b812b669d9a8509ce435ea9dcc4b37c))
- **config**: add structured hook configuration with pattern matching ([ba35d3c](https://github.com/urmzd/sr/commit/ba35d3c441c57471e3502d287c1ff8d3e97790ae))
- **release**: add v0 protection for breaking changes ([0c4641f](https://github.com/urmzd/sr/commit/0c4641ff4fcd04b39bdae565a7f0f2171668b49a))
- auto-detect version files in config and cli ([04d3341](https://github.com/urmzd/sr/commit/04d3341a7069e88d25f039dd90f4e8ba86e40436))
- **release**: auto-stage lock files after version bump ([9fbf2b0](https://github.com/urmzd/sr/commit/9fbf2b02453ea805eeb6b4abd559dc76d140d611))
- **version-files**: auto-discover and bump workspace members ([f792e4e](https://github.com/urmzd/sr/commit/f792e4e5a8595e05aa360e95a96cae55b0f8d296))
- **core**: add monorepo support with per-package releases ([7687587](https://github.com/urmzd/sr/commit/7687587b13324323e0719e3ff6a671886523ecf0))
- **git**: add head_short and file_statuses methods ([24b4f2a](https://github.com/urmzd/sr/commit/24b4f2a7a3a4aed7ee3ec81e577322c9cb58fa7b))
- **commands/commit**: add config-driven commit types and real-time event display ([950e882](https://github.com/urmzd/sr/commit/950e882c75e4abb0bfff1569079763f11fdf1395))
- **ai**: refactor backends with streaming and event support ([361ba23](https://github.com/urmzd/sr/commit/361ba23752f1ee0d47c0de43b988e2b85cb1f4f4))
- **ai**: add sr-core dependency to sr-ai ([ece500e](https://github.com/urmzd/sr/commit/ece500ef6203a828755eddeae05e8bd677715038))
- **cli**: add git hooks management, self-update, and remove pre-commit framework ([79ee5f5](https://github.com/urmzd/sr/commit/79ee5f5eb6b79ee5fd08fc182e5152d0d7cb7525))
- integrate AI-powered git commands into sr ([99dda91](https://github.com/urmzd/sr/commit/99dda91e961d660769099bc55e1c2f4c54a5f859))
- allow --force to create patch release when no releasable commits exist ([7c5364c](https://github.com/urmzd/sr/commit/7c5364c1d5925ee6104c4cce399faff3cd36f14b))
- improve release cycle with signing, drafts, checksums, PATCH updates, and more ([9222dc8](https://github.com/urmzd/sr/commit/9222dc818aac2a5337aef9705b3b47d79ec11af4))
- add pre-release support (alpha, beta, rc) ([2cae885](https://github.com/urmzd/sr/commit/2cae8850478c8e6c2498a41b833f2148bca50500))
- add changelog templates, stage_files, pre/post hooks, and rollback ([b63c315](https://github.com/urmzd/sr/commit/b63c3159b2a92d4fe85c13bbbc5485808111f44c))
- add build_command option and improve shell installer PATH setup ([1746685](https://github.com/urmzd/sr/commit/174668583ac9a11772824c1994a3eb42e0dd662c))
- add shell installer, Windows release target, and license housekeeping ([96d286f](https://github.com/urmzd/sr/commit/96d286f74d5fe0c9988a330bab2f772b8264bec4))
- replace gh CLI with direct GitHub REST API calls ([aec8284](https://github.com/urmzd/sr/commit/aec8284c21938178c158ef173224711e1b303416))
- prevent interactive git auth prompts and support CI runner credentials ([0b244ba](https://github.com/urmzd/sr/commit/0b244ba65fa5e9f937c27226f716017c82fd9b03))
- add GHES support, MUSL builds, ~/.local/bin install path, and post-release hooks docs ([dc1c74b](https://github.com/urmzd/sr/commit/dc1c74b394eba457d5bba3578df0b3003e4ff481))
- add --force flag, structured errors, distinct exit codes, and action outputs ([a20765d](https://github.com/urmzd/sr/commit/a20765dd5a7768f34509e98a1406e8ad969baee6))
- add floating major version tags support ([3fe4c0e](https://github.com/urmzd/sr/commit/3fe4c0e3c6cba9915eee3cae47e02071a31186fb))
- add release artifact upload support ([9665c27](https://github.com/urmzd/sr/commit/9665c274ae8ce0a1bfe3c9c2928b321ce11444eb))
- expand version file support, add lenient mode, fix root Cargo.toml ([184cbbc](https://github.com/urmzd/sr/commit/184cbbc82ee84d4fce4e5333b327b20945c017a0))
- tag contributors with GitHub @username in changelog ([6762b55](https://github.com/urmzd/sr/commit/6762b55a794b7f918145a0125b3b53d2b2e7b8b9))
- add changelog --regenerate flag and improve changelog sections ([5f1d4aa](https://github.com/urmzd/sr/commit/5f1d4aa9bfa83d734c63df6e2656c6178596c781))
- add hook context, version file bumping, and changelog SHA links ([0f7c759](https://github.com/urmzd/sr/commit/0f7c759bf1dbbfc88efe4e73db07d6b875604b93))
- add shell completions, enhance plan preview, fix dry-run hooks ([62486c2](https://github.com/urmzd/sr/commit/62486c2c2aa273af86eccfa103cd6fb3b155ff15))
- make release execution idempotent with CHANGELOG commit ([afabfd6](https://github.com/urmzd/sr/commit/afabfd63884308d3b1648ff6e224c8f6c965d864))
- initial implementation of semantic-release toolchain ([1cc191b](https://github.com/urmzd/sr/commit/1cc191b0a471b4f9ee93afaf4308fde9db3452fa))

### Bug Fixes

- **github**: increase body size limit for floating tag asset sync ([ae1d8fd](https://github.com/urmzd/sr/commit/ae1d8fdc56f23b157993ec1956e273b2235fc549))
- **ci**: build before release to sync floating tag assets ([d6b303b](https://github.com/urmzd/sr/commit/d6b303be63b92234c16fdc8f9962919f46be89c3))
- **readme**: correct crates.io badge to reference sr-cli ([6ab3729](https://github.com/urmzd/sr/commit/6ab37291cdfd6699eed26c69c84a4228a0308a11))
- sync floating tag releases with versioned release assets ([e34f304](https://github.com/urmzd/sr/commit/e34f304d786964cd374fbc174dcf9253d6a2981c))
- **git**: properly track both paths in file rename operations (#5) ([3f123b0](https://github.com/urmzd/sr/commit/3f123b04106aafdd7bdce8a6220e4cd5b53bd817))
- use lightweight tags and refactor hooks to core (#4) ([f4da72f](https://github.com/urmzd/sr/commit/f4da72f102dd006d679b23296b0b510ea46d6140))
- use lightweight tags for floating version tags (#3) ([a6b51ae](https://github.com/urmzd/sr/commit/a6b51ae42218f3245b99315af2baa019e8e8ca03))
- **commit**: track snapshot status during dry-run ([decef99](https://github.com/urmzd/sr/commit/decef996b3f497ee08872a73129bfde23a75fd67))
- **commit**: hide future-commit files from pre-commit hooks ([c9ec533](https://github.com/urmzd/sr/commit/c9ec533ad3392f9c85a37bb3870eef979a1579ee))
- **snapshot**: replace stash-based snapshots with direct file copies ([6d16925](https://github.com/urmzd/sr/commit/6d16925e69c878bafc2b56e72baeec6cc19c028f))
- **ci**: handle already-published crates in publish step ([57d6aeb](https://github.com/urmzd/sr/commit/57d6aeb645f9d856e473c254053e4db3f0d278bf))
- **ci**: checkout release tag for publish and handle re-publishes ([34b47b6](https://github.com/urmzd/sr/commit/34b47b697d865360ecbf613a2071f2bdbafa8272))
- make refactor commits trigger a patch release ([caa6db2](https://github.com/urmzd/sr/commit/caa6db2775a0ea034904aeec4c7eb5f333abb41d))
- remove target_commitish from release payload to avoid GitHub 422 error ([7afbe4d](https://github.com/urmzd/sr/commit/7afbe4d352b1e89a8ef711b398b8a9e09c6baf62))
- add Cargo.lock to stage_files and sync with v1.5.0 ([0f8b30b](https://github.com/urmzd/sr/commit/0f8b30b8a92ce02ba696b36c341d39f8253b0f94))
- use env var for build-command to avoid YAML/shell expansion issues ([3920736](https://github.com/urmzd/sr/commit/392073634c55ebe97aadb7ca8498fdced6c5cb0e))
- write build-command to temp file to prevent outer shell expansion ([ae52499](https://github.com/urmzd/sr/commit/ae5249951bebd33834fced9d8d96e493310dfb79))
- don't let crates.io publish failure block binary uploads ([fe0447d](https://github.com/urmzd/sr/commit/fe0447d1748a82bfe7e237a91bf8f80f9eef561d))
- add force re-release support to workflow dispatch ([520b029](https://github.com/urmzd/sr/commit/520b0293c2e087d3dc19f383700ee52355a7b8f1))
- trigger binary builds from release workflow ([ee1297b](https://github.com/urmzd/sr/commit/ee1297b148d76e75f67e337b7b78cd3c5556517c))
- clear existing git extraheader before injecting auth ([eb64d0a](https://github.com/urmzd/sr/commit/eb64d0ab8885ea07fe89c6732ac4dfaf2f0cde99))
- add floating_tags, parse JSON output in self-release workflow ([c51e6a9](https://github.com/urmzd/sr/commit/c51e6a9d3c13fe02970b68f1f7dc41cb01c4f30b))
- update workspace dependency versions during Cargo.toml version bump ([175509d](https://github.com/urmzd/sr/commit/175509d7ab8f656b069f29bee6ef94738774b4fc))
- use tempdir in release tests to prevent changelog file pollution ([817edac](https://github.com/urmzd/sr/commit/817edac9144e98a65412deea42024acf5ca68919))
- add skip-ci flag to release commit messages to avoid redundant CI runs ([5dba595](https://github.com/urmzd/sr/commit/5dba5954f2d76376b242547baaedc41b72f0d2af))
- configure git identity for tag creation in CI ([6e02cd9](https://github.com/urmzd/sr/commit/6e02cd9fbbe88f1b8dcff7ba7d7547834713ef69))

### Documentation

- **readme**: add crates.io version badge ([7d1f1b4](https://github.com/urmzd/sr/commit/7d1f1b416efc93008117cfe1ae858ca399459bf0))
- **skills**: align SKILL.md with agentskills.io spec ([64e1717](https://github.com/urmzd/sr/commit/64e1717158355b0bbc76513913309dd3aedebecc))
- **showcase**: update commit demo assets ([231605c](https://github.com/urmzd/sr/commit/231605c7214e91469b9db7eb081d2189f411e0c7))
- **readme**: add all showcase examples to README ([2bc3da8](https://github.com/urmzd/sr/commit/2bc3da8adf3c1dd9be2a5180902b3fa0592a07b3))
- **demo**: add hidden git reset step to demo script ([b5b01c9](https://github.com/urmzd/sr/commit/b5b01c9ba90fcc868e183e18ab13ad7c211e4738))
- **showcase**: update sr commit demo assets ([ff5ad06](https://github.com/urmzd/sr/commit/ff5ad060f2e4582a501569534c9243b05b7a20db))
- add sr rebase command and fix stale versions ([f331ce8](https://github.com/urmzd/sr/commit/f331ce869f02dd3ed2f09273d3eb48017beb535c))
- update hook synchronization documentation ([f78fedb](https://github.com/urmzd/sr/commit/f78fedb39e06f5870b1a530ebc05942791400fbe))
- update documentation for v2 release ([749842d](https://github.com/urmzd/sr/commit/749842d370b7928b09266c8d3177f179e0a76f76))
- add showcase section to README ([630a20a](https://github.com/urmzd/sr/commit/630a20a051adbb9497f3271247438643788f5844))
- add demonstration screenshots ([60f3004](https://github.com/urmzd/sr/commit/60f3004122a42738b2d25060ca95c5e1e8318270))
- add monorepo support documentation ([27de5b1](https://github.com/urmzd/sr/commit/27de5b1e9d29d183fa9c58df8e63e3f9ef14da31))
- update all documentation to reflect AI-powered CLI ([e517cd2](https://github.com/urmzd/sr/commit/e517cd2af61616b48471ef5f283eec8d5522db30))
- add AGENTS.md and agent skill for Claude Code ([05084ea](https://github.com/urmzd/sr/commit/05084ea18181f64c6aacc7a5f355f4bcaebf5a9b))
- remove incorrect single-branch limitation ([7d6ef21](https://github.com/urmzd/sr/commit/7d6ef21f1315d14711919c1bc7ae09f99b9a0c45))
- comprehensive configuration and version file documentation ([1c9eaa9](https://github.com/urmzd/sr/commit/1c9eaa942c846015e35b519823e5c09b6a531395))
- remove License section from README ([80dda52](https://github.com/urmzd/sr/commit/80dda523dfab9e673a5f2d62da5503bad0d3fd75))
- regenerate changelog with GitHub @username contributors ([40e4714](https://github.com/urmzd/sr/commit/40e471448e194175014374871a3fb061db7c3059))
- update README with version files, completions, and developer workflow ([9de2e73](https://github.com/urmzd/sr/commit/9de2e739cc4b02048ab6047f37bcb91629e2c432))
- add action usage examples, branding, and marketplace metadata ([77c9955](https://github.com/urmzd/sr/commit/77c9955c8d798f0f2f0dbb6c2ebd1f9ad4043e3d))

### Refactoring

- **core**: improve release strategy design for maintainability ([2f6484f](https://github.com/urmzd/sr/commit/2f6484fbdf7bd9784e3e96b9b47f2afc29103b79))
- **commands**: extract rebase command from commit ([72d7446](https://github.com/urmzd/sr/commit/72d74465745f198d88ee9655307032ca7a2dd271))
- **test**: use struct literal for ReleaseConfig initialization ([d0771e0](https://github.com/urmzd/sr/commit/d0771e03abc16b8dbf54ae8d5991b878812d7a47))
- **hooks**: extract commit-msg logic to rust library ([ccb92dd](https://github.com/urmzd/sr/commit/ccb92dddf71122ce47ebdc68edab87ac69aabcae))
- **release**: simplify supported file check using new abstraction ([c4ff9f7](https://github.com/urmzd/sr/commit/c4ff9f7428fde9ccaafc9f15aa7663b822ed5a3c))
- **core**: introduce VersionFileHandler trait for version file handling ([324d51c](https://github.com/urmzd/sr/commit/324d51cf9df99bd5cab24e4005a3da532782c50f))
- **ui**: redesign output with improved styling and interactive progress ([df06ba8](https://github.com/urmzd/sr/commit/df06ba8b75448b75faad2641fb68e402f4eeb80f))
- **commands**: update backend request calls for new event parameter ([100f7e7](https://github.com/urmzd/sr/commit/100f7e732ee83c241be0a02a0e34808f1730106c))
- rename repo references from urmzd/semantic-release to urmzd/sr ([585b08f](https://github.com/urmzd/sr/commit/585b08f112c4a40b2847e57b7192608fe21c6c58))
- rename config to sr.yaml, remove toml/json support ([00b96f1](https://github.com/urmzd/sr/commit/00b96f160993b7ce32d865fccd0c2e06e1b7a527))
- remove contributors section from release notes ([2ff6093](https://github.com/urmzd/sr/commit/2ff6093676aa4609a18786e3607a9b40ebb40ccc))
- replace git-conventional with built-in regex commit parser ([71553d9](https://github.com/urmzd/sr/commit/71553d9462f9ddb3066325724f5e36b2998eb10b))
- replace octocrab with gh CLI, fix macOS runners, move action.yml to root ([1d97cc0](https://github.com/urmzd/sr/commit/1d97cc0a791dd91fdf2e4435fdcbc14e427949e9))

### Reverts

- **commit**: remove file-hiding approach for pre-commit hooks ([7779658](https://github.com/urmzd/sr/commit/7779658ed41254daa0074071f300499ad3213d3c))

### Miscellaneous

- **deps**: bump actions/checkout from 4 to 6 ([0d62674](https://github.com/urmzd/sr/commit/0d626744d08e70122ccf557894a2d844c6f43455))
- use sr-releaser GitHub App for release workflow (#2) ([86d70fa](https://github.com/urmzd/sr/commit/86d70fa5699de95abd90768d774bb6cf9703992a))
- **deps**: bump sr-ai to 2.3.2 and add regex dependency ([ae8fa6d](https://github.com/urmzd/sr/commit/ae8fa6db12bba23b8932d64428be972461254ffc))
- **snapshot**: add snapshot/restore integration tests ([39a6d8e](https://github.com/urmzd/sr/commit/39a6d8e57b2e265861f080fd98edfe448e4f5161))
- **justfile**: add record and ci recipes ([ee8e925](https://github.com/urmzd/sr/commit/ee8e92599c10b358ca046644c2d3a9d7b96572ce))
- remove backup commit-msg hook and ignore *.bak files ([ef4d6d5](https://github.com/urmzd/sr/commit/ef4d6d57bd274a1082ee8b1132606fb04720b4c6))
- **build**: integrate hook setup into justfile ([e2b2dd8](https://github.com/urmzd/sr/commit/e2b2dd8a8e0e9bc969e05abeb901f79e79faba7e))
- ignore machine-local hook sync state ([26705cd](https://github.com/urmzd/sr/commit/26705cdf4ca1efe8f27a14a1d64a574aa937b3de))
- document sr.yaml with comprehensive inline comments ([5abb949](https://github.com/urmzd/sr/commit/5abb949fe722d49cc475f35787459a052bc6b9c1))
- update GitHub Action metadata and build configuration ([791d66b](https://github.com/urmzd/sr/commit/791d66b72bf0fa86ee380ccbf317088d29864800))
- **showcase**: optimize demo images ([3ea66dc](https://github.com/urmzd/sr/commit/3ea66dc646020e13902313b4123521b4d1d192b2))
- **lock**: update Cargo.lock for v2.1.0 ([a3661c8](https://github.com/urmzd/sr/commit/a3661c8c7accf948ab4e25815719c9eab2751207))
- **showcase**: update demo screenshots and animation ([3f9e37a](https://github.com/urmzd/sr/commit/3f9e37afd905719e618b2330200185abfc9d550f))
- **config**: enhance teasr demo configuration ([ff75590](https://github.com/urmzd/sr/commit/ff75590021f48a24d97570815e4af83d820c5895))
- update teasr configuration ([5eca457](https://github.com/urmzd/sr/commit/5eca457f4f55aa4cb421816c3561ef89d75e6001))
- standardize project files and README header ([e2b9920](https://github.com/urmzd/sr/commit/e2b992088900230ad5aea4286c5abe1d90a5644b))
- switch to trusted publishing for crates.io ([79fb2cc](https://github.com/urmzd/sr/commit/79fb2cc42716bf37e27a60465addd88c274874ca))
- standardize GitHub Actions workflows ([d6daf57](https://github.com/urmzd/sr/commit/d6daf57637a801e0dac4c7a2e761e4019ddcc123))
- move crates.io publish to separate job so build is never blocked ([894dc4c](https://github.com/urmzd/sr/commit/894dc4cb246d4dba106f0f539f6561743497dc35))
- lock file update ([3221762](https://github.com/urmzd/sr/commit/3221762021db266159ab37323bb91c5ccd4a771a))
- inline build matrix into release.yml, remove build.yml ([16610b1](https://github.com/urmzd/sr/commit/16610b13b6abf3a01ac8d780608ba5e90574cb36))
- add sensitive paths to .gitignore ([af72f4b](https://github.com/urmzd/sr/commit/af72f4bc33f7e4719e565782fdb09c0b844850d0))
- cleanup cargo.lock ([0438671](https://github.com/urmzd/sr/commit/043867120908210776e2ddd0e7f7a111eeb88c5e))
- update README examples to reference v1 ([d7113b9](https://github.com/urmzd/sr/commit/d7113b92ce7e2ef0d9c02adef1c5c0f53f37e922))
- update Cargo.toml license to Apache-2.0 ([621cc4e](https://github.com/urmzd/sr/commit/621cc4eb94afa5c77d51e7f15eda5d044f373bf0))
- license under Apache 2.0 ([3b6a267](https://github.com/urmzd/sr/commit/3b6a2677f4fd05f127e7a311c935c6cc70a8fa64))
- fix clippy warnings, apply cargo fmt, and fix commit-msg hook PCRE parsing ([ebee586](https://github.com/urmzd/sr/commit/ebee586f0af99fdc2e4ab8afa11f680d3a5d0836))
- update Cargo.lock for v0.7.0 ([65aa670](https://github.com/urmzd/sr/commit/65aa670850d6370e068e9ba920249c3194133ed2))
- add floating v0 tag, update docs, and prepare crates.io publishing ([194dfaa](https://github.com/urmzd/sr/commit/194dfaa8bfaf576bc9087d227ca16dcefff6a3d2))
- fix formatting in changelog contributor rendering ([672001a](https://github.com/urmzd/sr/commit/672001ae44fd6e62da74553e21c3dd07adb53e72))
- remove redundant push trigger from CI workflow ([8b9ef8d](https://github.com/urmzd/sr/commit/8b9ef8d008ca9cc44d66c9bc80b9eabe430c43d7))
- fix formatting and clippy warnings, add pre-commit hook ([54f4a1b](https://github.com/urmzd/sr/commit/54f4a1b5a36f4a676efa3bc370230f8f2353ee6b))
- fix cargo fmt formatting in sr-github ([116d22b](https://github.com/urmzd/sr/commit/116d22be426c91dc1e0e0a25db9c7be251be928b))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.8...v3.0.0)


## 2.4.7 (2026-03-30)

### Bug Fixes

- **ci**: build before release to sync floating tag assets ([3f299a9](https://github.com/urmzd/sr/commit/3f299a90667c0e8f6be8d51a41f7234b962e2855))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.6...v2.4.7)


## 2.4.6 (2026-03-30)

### Bug Fixes

- **readme**: correct crates.io badge to reference sr-cli ([51a9d79](https://github.com/urmzd/sr/commit/51a9d79f102feebc3e301f6ca676ac9d066ace48))

### Documentation

- **readme**: add crates.io version badge ([ee0d325](https://github.com/urmzd/sr/commit/ee0d3252e27e3e7aea1cab9859c425a2801111c8))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.5...v2.4.6)


## 2.4.5 (2026-03-30)

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.5...v2.4.5)


## 2.4.5 (2026-03-30)

### Bug Fixes

- sync floating tag releases with versioned release assets ([e6b41e2](https://github.com/urmzd/sr/commit/e6b41e22444bb73a276731c442c863bad517c7c3))

### Miscellaneous

- **deps**: bump actions/checkout from 4 to 6 ([7c86a5c](https://github.com/urmzd/sr/commit/7c86a5c0c8d7c712db9a5f024ee6683d786e7f69))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.4...v2.4.5)


## 2.4.4 (2026-03-29)

### Bug Fixes

- **git**: properly track both paths in file rename operations (#5) ([4bb39e8](https://github.com/urmzd/sr/commit/4bb39e8580797458c1016b1ac32149c91a5658b6))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.3...v2.4.4)


## 2.4.3 (2026-03-27)

### Bug Fixes

- use lightweight tags and refactor hooks to core (#4) ([8b87757](https://github.com/urmzd/sr/commit/8b877570aea0e5ba29d5acf02fa0beb52d1debf9))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.2...v2.4.3)


## 2.4.2 (2026-03-26)

### Bug Fixes

- use lightweight tags for floating version tags (#3) ([75a1312](https://github.com/urmzd/sr/commit/75a1312ff36be31cdfc6e143eaecd3120f4c67e4))

### Documentation

- **skills**: align SKILL.md with agentskills.io spec ([0344b51](https://github.com/urmzd/sr/commit/0344b5185807e77b726dd0f1c6d6d889d90989f2))

### Miscellaneous

- use sr-releaser GitHub App for release workflow (#2) ([47d63ba](https://github.com/urmzd/sr/commit/47d63bab5e3df103f2037df359369810e8dda36b))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.1...v2.4.2)


## 2.4.1 (2026-03-23)

### Bug Fixes

- **commit**: track snapshot status during dry-run ([3ca64c5](https://github.com/urmzd/sr/commit/3ca64c53054266cf2e1492a20241691cea7b7fa5))

### Documentation

- **showcase**: update commit demo assets ([552f539](https://github.com/urmzd/sr/commit/552f53962fd24f7a801c7e4850f49dada9377a64))
- **readme**: add all showcase examples to README ([0a0b1b7](https://github.com/urmzd/sr/commit/0a0b1b738e72b7fdc6d19d054c224b7d9bdea816))
- **demo**: add hidden git reset step to demo script ([9bec5cf](https://github.com/urmzd/sr/commit/9bec5cf14b2ec69dca035cb9162b2fa38ea5cf41))
- **showcase**: update sr commit demo assets ([c505c3f](https://github.com/urmzd/sr/commit/c505c3fc30c5d40ecb5d7024afee39f9c1aa8ed3))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.4.0...v2.4.1)


## 2.4.0 (2026-03-23)

### Features

- **commit**: validate messages before execution with error recovery ([34ce2a6](https://github.com/urmzd/sr/commit/34ce2a6f1200bea164315cb5c275489562d70fec))
- **ui**: add commit validation and failure display functions ([d51b50d](https://github.com/urmzd/sr/commit/d51b50d9b4091a5ed09c69bcb98ca74af04b5122))

### Reverts

- **commit**: remove file-hiding approach for pre-commit hooks ([5dc8c52](https://github.com/urmzd/sr/commit/5dc8c52b161db812cd5c0edb6676a48aae1b3cfd))

### Miscellaneous

- **deps**: bump sr-ai to 2.3.2 and add regex dependency ([7e83439](https://github.com/urmzd/sr/commit/7e83439f8a0b0371db53fc839530bae4892e48ac))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.3.2...v2.4.0)


## 2.3.2 (2026-03-23)

### Bug Fixes

- **commit**: hide future-commit files from pre-commit hooks ([c47df24](https://github.com/urmzd/sr/commit/c47df247ed04b24c28b521a826b4b7a691bf7728))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.3.1...v2.3.2)


## 2.3.1 (2026-03-23)

### Bug Fixes

- **snapshot**: replace stash-based snapshots with direct file copies ([898fb67](https://github.com/urmzd/sr/commit/898fb67d59998e112d7e9dfaaf5c75c5575c80b1))

### Documentation

- add sr rebase command and fix stale versions ([038ba35](https://github.com/urmzd/sr/commit/038ba3594aee9756d91ee908be1d18eba2db1ca8))

### Miscellaneous

- **snapshot**: add snapshot/restore integration tests ([42742c6](https://github.com/urmzd/sr/commit/42742c64bb431bb08582e5433483ac3e24954396))
- **justfile**: add record and ci recipes ([0f86f98](https://github.com/urmzd/sr/commit/0f86f9841f096aed18fa246b8fce133877f62c9c))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.3.0...v2.3.1)


## 2.3.0 (2026-03-22)

### Features

- **commit**: add commit reorganization functionality ([4e2dc8e](https://github.com/urmzd/sr/commit/4e2dc8e98d0bc8dda9c04aa0e1813374a57caffc))
- **git**: add commits_since_last_tag method ([92d1f2c](https://github.com/urmzd/sr/commit/92d1f2c879dd59985bcbbd674099e06c8fed91c3))
- **cli**: implement automatic hook synchronization ([66336a9](https://github.com/urmzd/sr/commit/66336a9d91e85374a43bc732e5f477a02db6a8c3))
- **cli**: implement --merge flag for incremental config updates ([c39360e](https://github.com/urmzd/sr/commit/c39360e5863d2044a898212f9aad982fda2af2cd))
- **config**: add structured hook configuration with pattern matching ([bd6d8d9](https://github.com/urmzd/sr/commit/bd6d8d9bb6228a569bf2e86e0f166d8474ad0f5a))

### Documentation

- update hook synchronization documentation ([633d54c](https://github.com/urmzd/sr/commit/633d54c78e57f8729a702be378291e062354c37b))
- update documentation for v2 release ([1564679](https://github.com/urmzd/sr/commit/1564679541e9773c1f39fbc02d642f6ef6638963))

### Refactoring

- **commands**: extract rebase command from commit ([f78d959](https://github.com/urmzd/sr/commit/f78d959dc671d7a988842644888acc350caeaf59))
- **test**: use struct literal for ReleaseConfig initialization ([3da50ff](https://github.com/urmzd/sr/commit/3da50ffb55d3536539c893a81ca4477ce26f0900))
- **hooks**: extract commit-msg logic to rust library ([1e7f6dd](https://github.com/urmzd/sr/commit/1e7f6ddc93f26ab18a30674ff3edbac6cb768de1))

### Miscellaneous

- remove backup commit-msg hook and ignore *.bak files ([dbb8973](https://github.com/urmzd/sr/commit/dbb8973c52a587c2c58009138d31850eae2af08e))
- **build**: integrate hook setup into justfile ([9c42be3](https://github.com/urmzd/sr/commit/9c42be34b2abe4a570b6b74723d9939eebd7af46))
- ignore machine-local hook sync state ([5c41463](https://github.com/urmzd/sr/commit/5c41463e309002800103fc73920f170458263a9b))
- document sr.yaml with comprehensive inline comments ([e08a15d](https://github.com/urmzd/sr/commit/e08a15d1d791a0a4e5e4c921d38c80916dd6ac88))
- update GitHub Action metadata and build configuration ([8620d73](https://github.com/urmzd/sr/commit/8620d739a65fddb73efa67ff6d38378eb102e02c))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.2.0...v2.3.0)


## 2.2.0 (2026-03-22)

### Features

- **release**: add v0 protection for breaking changes ([be1311f](https://github.com/urmzd/sr/commit/be1311fed03a6e78a50c4ef165bbd6f79bd747a2))

### Documentation

- add showcase section to README ([a84de47](https://github.com/urmzd/sr/commit/a84de47d983938f45adc8c69f32572392e9a76f2))

### Miscellaneous

- **showcase**: optimize demo images ([eb92755](https://github.com/urmzd/sr/commit/eb92755329fed9031310569dcabb2f14fbcc059d))
- **lock**: update Cargo.lock for v2.1.0 ([d83d0c4](https://github.com/urmzd/sr/commit/d83d0c42bb0b6ba4baf31b2602e94d5be7446130))
- **showcase**: update demo screenshots and animation ([ec7bbf1](https://github.com/urmzd/sr/commit/ec7bbf111ac8fdd7a65cb999a210e4071f83e9bc))
- **config**: enhance teasr demo configuration ([00e653e](https://github.com/urmzd/sr/commit/00e653e21d93caf8a12a49efd49ab415d25d06f8))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.1.0...v2.2.0)


## 2.1.0 (2026-03-21)

### Features

- auto-detect version files in config and cli ([05d6f82](https://github.com/urmzd/sr/commit/05d6f82f2af810662946dd8f3dfc8bdb22e4c422))

### Documentation

- add demonstration screenshots ([9d78bd5](https://github.com/urmzd/sr/commit/9d78bd5e2cac5a93119770a455f157e207a29f57))

### Refactoring

- **release**: simplify supported file check using new abstraction ([452900b](https://github.com/urmzd/sr/commit/452900bd29e2b0fca3c9ef175390985c7e6f0535))
- **core**: introduce VersionFileHandler trait for version file handling ([c1c6620](https://github.com/urmzd/sr/commit/c1c66208590d51955b20f2e964ffc12ce9c15cc4))

### Miscellaneous

- update teasr configuration ([ff22537](https://github.com/urmzd/sr/commit/ff22537b1419996864d48c090deb196ef6eaccea))

[Full Changelog](https://github.com/urmzd/sr/compare/v2.0.0...v2.1.0)


## 2.0.0 (2026-03-21)

### Breaking Changes

- **ai**: sandbox agent backends to read-only git access with working tree snapshots ([a8937ed](https://github.com/urmzd/sr/commit/a8937ed34df8f5947b6eac6e951ec207e0e6bb2b))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.13.0...v2.0.0)


## 1.13.0 (2026-03-20)

### Features

- **release**: auto-stage lock files after version bump ([8a71452](https://github.com/urmzd/sr/commit/8a71452db9979f2981b5b75ec24c29fc279d9596))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.12.0...v1.13.0)


## 1.12.0 (2026-03-20)

### Features

- **version-files**: auto-discover and bump workspace members ([b28c18e](https://github.com/urmzd/sr/commit/b28c18e6426a4fc077d984b69ff4e7f9179f237b))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.11.0...v1.12.0)


## 1.11.0 (2026-03-20)

### Features

- **core**: add monorepo support with per-package releases ([6b56089](https://github.com/urmzd/sr/commit/6b56089d3d0bd2576e19ff62f29156aef7a795e7))

### Documentation

- add monorepo support documentation ([ef78791](https://github.com/urmzd/sr/commit/ef78791d1a509b0bf5eefb306a3f88c9661d1820))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.10.0...v1.11.0)


## 1.10.0 (2026-03-20)

### Features

- **git**: add head_short and file_statuses methods ([e512a4c](https://github.com/urmzd/sr/commit/e512a4c2b7211e1f001191b85c691209ab4a8c78))
- **commands/commit**: add config-driven commit types and real-time event display ([deb0ea8](https://github.com/urmzd/sr/commit/deb0ea849a8892b3c9ba86209f48bc851f88c2a1))
- **ai**: refactor backends with streaming and event support ([4bed74f](https://github.com/urmzd/sr/commit/4bed74fd185942c6b10d0baeb095b5145294fc98))
- **ai**: add sr-core dependency to sr-ai ([c9e2e38](https://github.com/urmzd/sr/commit/c9e2e38c5185a665bf5014d9f5c0b4b5d3bae882))

### Refactoring

- **ui**: redesign output with improved styling and interactive progress ([a679810](https://github.com/urmzd/sr/commit/a679810ba67bd3e562396e1fa1ca3dfd787b07e7))
- **commands**: update backend request calls for new event parameter ([6877e1e](https://github.com/urmzd/sr/commit/6877e1e39798a2df3b2c143372e6aa81f951eb54))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.9.0...v1.10.0)


## 1.9.0 (2026-03-19)

### Features

- **cli**: add git hooks management, self-update, and remove pre-commit framework ([35460d5](https://github.com/urmzd/sr/commit/35460d50513d448252bd2beaa85553f65f5c1317))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.8.2...v1.9.0)


## 1.8.2 (2026-03-19)

### Bug Fixes

- **ci**: handle already-published crates in publish step ([4b2c8d1](https://github.com/urmzd/sr/commit/4b2c8d1da5ae5fcacbec5dbe73ddec1d0f9b2eee))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.8.1...v1.8.2)


## 1.8.1 (2026-03-19)

### Bug Fixes

- **ci**: checkout release tag for publish and handle re-publishes ([e5e40dd](https://github.com/urmzd/sr/commit/e5e40ddfd7c9cf6456cdb32ee6a9b39d2adde435))

### Documentation

- update all documentation to reflect AI-powered CLI ([6271e9e](https://github.com/urmzd/sr/commit/6271e9ec6b648444eb2d2fa86f4080d211a71222))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.8.0...v1.8.1)


## 1.8.0 (2026-03-19)

### Features

- integrate AI-powered git commands into sr ([0c333ab](https://github.com/urmzd/sr/commit/0c333ab568818dc6d59195541bdf608ce1ac0bd0))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.7.2...v1.8.0)


## 1.7.2 (2026-03-16)

### Refactoring

- rename repo references from urmzd/semantic-release to urmzd/sr ([68659ca](https://github.com/urmzd/sr/commit/68659caa5a61b9b14c0efbb0627171b531d54085))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.7.1...v1.7.2)


## 1.7.1 (2026-03-16)

### Bug Fixes

- make refactor commits trigger a patch release ([290e96e](https://github.com/urmzd/sr/commit/290e96e4d449b2b1ffead4351b2f9a06d6f0ed65))

### Refactoring

- rename config to sr.yaml, remove toml/json support ([ad1c6f2](https://github.com/urmzd/sr/commit/ad1c6f2e0b861aa1c670f96b717784e26c5a91d1))

### Miscellaneous

- standardize project files and README header ([415e32c](https://github.com/urmzd/sr/commit/415e32c9f43fc4b22eddfc8b0e03502f0b89fe78))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.7.0...v1.7.1)


## 1.7.0 (2026-03-14)

### Features

- allow --force to create patch release when no releasable commits exist ([4968d8b](https://github.com/urmzd/sr/commit/4968d8b8a26f3b282124f1a30b462dce50e29dde))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.6.1...v1.7.0)


## 1.6.1 (2026-03-14)

### Bug Fixes

- remove target_commitish from release payload to avoid GitHub 422 error ([973ea38](https://github.com/urmzd/sr/commit/973ea38b115d5174d13c3a5caf4c18d4d3aa2706))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.6.0...v1.6.1)


## 1.6.0 (2026-03-14)

### Features

- improve release cycle with signing, drafts, checksums, PATCH updates, and more ([a23a210](https://github.com/urmzd/sr/commit/a23a210fea1111e5d80b9bc2d869741f1a1611dd))

### Documentation

- add AGENTS.md and agent skill for Claude Code ([3ad54b0](https://github.com/urmzd/sr/commit/3ad54b08888edfa6120af085f2d6b8c0703ed9c8))

[Full Changelog](https://github.com/urmzd/sr/compare/v1.5.1...v1.6.0)


## 1.5.1 (2026-03-10)

### Bug Fixes

- add Cargo.lock to stage_files and sync with v1.5.0 ([5463b67](https://github.com/urmzd/sr/commit/5463b67d9178c311099cc76db7b2e71e8c61138d))

### Documentation

- remove incorrect single-branch limitation ([36d18a8](https://github.com/urmzd/sr/commit/36d18a82b9bbf4729262657a8f5d5820af854e0c))


## 1.5.0 (2026-03-10)

### Features

- add pre-release support (alpha, beta, rc) ([f33b949](https://github.com/urmzd/sr/commit/f33b9492fdd912e91f23b3eaead21cc14a85e9ee))
- add changelog templates, stage_files, pre/post hooks, and rollback ([93c524a](https://github.com/urmzd/sr/commit/93c524a686090d11dd1810dee1bc105cde063743))

### Documentation

- comprehensive configuration and version file documentation ([1ebcf81](https://github.com/urmzd/sr/commit/1ebcf81832575c57e57d81f1e8667d2c12c116c1))

### Miscellaneous

- switch to trusted publishing for crates.io ([ddb0be8](https://github.com/urmzd/sr/commit/ddb0be8fb1e2c237fb6de6194d3cc91d0818e96c))
- standardize GitHub Actions workflows ([eae04e5](https://github.com/urmzd/sr/commit/eae04e58d2afc7e6e07b3441218844a2038969ce))


## 1.4.4 (2026-02-25)

### Bug Fixes

- use env var for build-command to avoid YAML/shell expansion issues ([dd1f8fd](https://github.com/urmzd/sr/commit/dd1f8fd92433dd95d8ec7d300bac4d7d477af74b))


## 1.4.3 (2026-02-25)

### Bug Fixes

- write build-command to temp file to prevent outer shell expansion ([731f774](https://github.com/urmzd/sr/commit/731f774e67fa916805095db13f7969e210a6aa06))

### Miscellaneous

- move crates.io publish to separate job so build is never blocked ([7cf95c1](https://github.com/urmzd/sr/commit/7cf95c1a7a3c328e74d45cd7a574fc370570c023))
- lock file update ([d3f3f73](https://github.com/urmzd/sr/commit/d3f3f7370c22e73139522b639510513100fe5c4b))


## 1.4.2 (2026-02-25)

### Bug Fixes

- don't let crates.io publish failure block binary uploads ([5c0275f](https://github.com/urmzd/sr/commit/5c0275ff5748a0692a919cc56afd5f2c16029ed1))


## 1.4.1 (2026-02-25)

### Bug Fixes

- add force re-release support to workflow dispatch ([bf0b8e3](https://github.com/urmzd/sr/commit/bf0b8e33283993120ec99a1cd5324ec61ba24357))


## 1.4.0 (2026-02-25)

### Features

- add build_command option and improve shell installer PATH setup ([df32798](https://github.com/urmzd/sr/commit/df3279820d7065496eaeac6862240d8d515a9c78))

### Bug Fixes

- trigger binary builds from release workflow ([787cf1e](https://github.com/urmzd/sr/commit/787cf1e89909f2b696ccf768cfbb290eb9408d0b))

### Documentation

- remove License section from README ([e5d24c1](https://github.com/urmzd/sr/commit/e5d24c124acbb0d1f6ecd1d7531bc35c26024a54))

### Miscellaneous

- inline build matrix into release.yml, remove build.yml ([afc423f](https://github.com/urmzd/sr/commit/afc423f4fb50e7ee97a6f7db8f9f63726265227d))
- add sensitive paths to .gitignore ([742bc42](https://github.com/urmzd/sr/commit/742bc4257510b0b5d68eeb485939f2139dd5d5b4))


## 1.3.0 (2026-02-21)

### Features

- add shell installer, Windows release target, and license housekeeping ([039c9fe](https://github.com/urmzd/sr/commit/039c9fe3a1d45ec348cf53b27feda6af3bae8acf))

### Miscellaneous

- cleanup cargo.lock ([f4097bf](https://github.com/urmzd/sr/commit/f4097bf5afe1c798a07773ab4e68fbb1f46e8eb7))


## 1.2.0 (2026-02-19)

### Features

- replace gh CLI with direct GitHub REST API calls ([c04c82c](https://github.com/urmzd/sr/commit/c04c82c48474ee458dae376ce5d70fbf96385977))
- prevent interactive git auth prompts and support CI runner credentials ([5f19d00](https://github.com/urmzd/sr/commit/5f19d00e8ed0845100439b01666c4484e9066c1b))

### Bug Fixes

- clear existing git extraheader before injecting auth ([028e940](https://github.com/urmzd/sr/commit/028e940dd9fe84f3d5157a7d45e8bcc1f490a377))


## 1.1.0 (2026-02-18)

### Features

- add GHES support, MUSL builds, ~/.local/bin install path, and post-release hooks docs ([32cdafa](https://github.com/urmzd/sr/commit/32cdafadcc8b617c02c360b51e564a2437cbba96))

### Miscellaneous

- update README examples to reference v1 ([bf6e998](https://github.com/urmzd/sr/commit/bf6e9986b88c04e31605613d61f1fde8a5763795))
- update Cargo.toml license to Apache-2.0 ([b41d16a](https://github.com/urmzd/sr/commit/b41d16ac4b2afcfacbb69dc0b848eb945a08607f))
- license under Apache 2.0 ([039c7fb](https://github.com/urmzd/sr/commit/039c7fb4edeedbaced90999a85230700410df7f0))


## 1.0.0 (2026-02-11)

### Breaking Changes

- remove lifecycle hooks, output structured JSON from sr release ([56e5ed0](https://github.com/urmzd/sr/commit/56e5ed0125c64455129f40a15ab70950aeb2e349))

### Bug Fixes

- add floating_tags, parse JSON output in self-release workflow ([70e0f04](https://github.com/urmzd/sr/commit/70e0f04f4b5abbd621821637c78c851ede770ac5))


## 0.10.0 (2026-02-11)

### Features

- add --force flag, structured errors, distinct exit codes, and action outputs ([ea0ba9c](https://github.com/urmzd/sr/commit/ea0ba9cd416465ad5d6d09642b517f232756f557))

### Refactoring

- remove contributors section from release notes ([2831757](https://github.com/urmzd/sr/commit/2831757c0dd85cf63958bf212b4288e229bfffa8))


## 0.9.0 (2026-02-09)

### Features

- add floating major version tags support ([ece1331](https://github.com/urmzd/sr/commit/ece133156582e5fd13a36027e630a524fe2cfa91))

### Contributors

- @urmzd


## 0.8.0 (2026-02-08)

### Features

- add release artifact upload support ([3fa9ea7](https://github.com/urmzd/sr/commit/3fa9ea720e48af6728c799c251c3075bf944a6f7))

### Bug Fixes

- update workspace dependency versions during Cargo.toml version bump ([c7a6634](https://github.com/urmzd/sr/commit/c7a66349a828905f5ab473c9490bd01f035cb088))

### Miscellaneous

- fix clippy warnings, apply cargo fmt, and fix commit-msg hook PCRE parsing ([2611a1c](https://github.com/urmzd/sr/commit/2611a1caf59717a91cf9d005646582b59c95ea57))
- update Cargo.lock for v0.7.0 ([e56772c](https://github.com/urmzd/sr/commit/e56772cde97f758146d0b8dc50f4ad42786fd8fa))

### Contributors

- @urmzd


## 0.7.0 (2026-02-08)

### Features

- expand version file support, add lenient mode, fix root Cargo.toml ([487e5f4](https://github.com/urmzd/sr/commit/487e5f4aa21a1d9e22298efebe310be0f193297c))

### Bug Fixes

- use tempdir in release tests to prevent changelog file pollution ([52e854d](https://github.com/urmzd/sr/commit/52e854d2ccc13dea844d6cfe74e8e233063c24f8))
- add skip-ci flag to release commit messages to avoid redundant CI runs ([87160a8](https://github.com/urmzd/sr/commit/87160a89b2650d52cacfd23db6c46dcaa3c7ffa8))

### Miscellaneous

- add floating v0 tag, update docs, and prepare crates.io publishing ([bda72c2](https://github.com/urmzd/sr/commit/bda72c2242f9119c99ff8850736c64255fa2ff0b))

### Contributors

- @urmzd


## 0.6.0 (2026-02-08)

### Features

- tag contributors with GitHub @username in changelog ([bc13a08](https://github.com/urmzd/sr/commit/bc13a08f8e43eafa9526dbb0607cee9efb3f81b9))

### Documentation

- regenerate changelog with GitHub @username contributors ([1d2906a](https://github.com/urmzd/sr/commit/1d2906a7f71f53c1eb72865bd5cf0c14afbf49aa))

### Miscellaneous

- fix formatting in changelog contributor rendering ([2276d07](https://github.com/urmzd/sr/commit/2276d07facda43f6ab56c32228e47db16f7f7634))

### Contributors

- @urmzd


## 0.5.0 (2026-02-08)

### Features

- add changelog --regenerate flag and improve changelog sections ([799c0f7](https://github.com/urmzd/sr/commit/799c0f7854956f79554e712bfb07beccf6a83c47))

### Miscellaneous

- remove redundant push trigger from CI workflow ([4ece192](https://github.com/urmzd/sr/commit/4ece1927a60234a4335eaf5aef97a691d14e159d))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.4.0...v0.5.0)

## 0.4.0 (2026-02-08)

### Features

- add hook context, version file bumping, and changelog SHA links ([d95ff03](https://github.com/urmzd/sr/commit/d95ff0373361fc0b04116f9a157f5dd121e76f36))

### Documentation

- update README with version files, completions, and developer workflow ([87fe902](https://github.com/urmzd/sr/commit/87fe90200d3dd965e00aa2a3236aa8fe7fc1e62b))

### Refactoring

- replace git-conventional with built-in regex commit parser ([a687a1c](https://github.com/urmzd/sr/commit/a687a1c4974d0215866f9f304b92ee975324455f))

### Miscellaneous

- fix formatting and clippy warnings, add pre-commit hook ([5bb14a9](https://github.com/urmzd/sr/commit/5bb14a98cf3ff2f9e25d26681deed614aac1eca0))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.3.0...v0.4.0)

## 0.3.0 (2026-02-08)

### Features

- add shell completions, enhance plan preview, fix dry-run hooks ([9fd9483](https://github.com/urmzd/sr/commit/9fd9483b4e5af9600c2b22f18f558bc588c63e7b))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.2.0...v0.3.0)

## 0.2.0 (2026-02-08)

### Features

- make release execution idempotent with CHANGELOG commit ([c78a302](https://github.com/urmzd/sr/commit/c78a302584b3ad64ca57d7d1252639324a9e50ac))

### Contributors

- @urmzd

[Full Changelog](https://github.com/urmzd/sr/compare/v0.1.0...v0.2.0)

## 0.1.0 (2026-02-07)

### Features

- initial implementation of semantic-release toolchain ([45eaa61](https://github.com/urmzd/sr/commit/45eaa6164e797045855f056ecd88e15b5dd08437))

### Bug Fixes

- configure git identity for tag creation in CI ([3d76f38](https://github.com/urmzd/sr/commit/3d76f38211cd0c5b78de9b937cc317fae57cd302))

### Documentation

- add action usage examples, branding, and marketplace metadata ([38a3ed3](https://github.com/urmzd/sr/commit/38a3ed306dc2e7865937d74375a1fb3e15317365))

### Refactoring

- replace octocrab with gh CLI, fix macOS runners, move action.yml to root ([b66bb29](https://github.com/urmzd/sr/commit/b66bb29663bda335d3a18607cf22457f2faf4132))

### Miscellaneous

- fix cargo fmt formatting in sr-github ([ed3b56c](https://github.com/urmzd/sr/commit/ed3b56ca781cdf1ce8e14fce26f2bce2955d11b8))

### Contributors

- @urmzd
