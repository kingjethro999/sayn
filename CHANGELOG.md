# Changelog

All notable changes to the **Sayn** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and follows Sayn's commit-driven version rollover rules.

## [0.0.5] - 2026-10-28

### Added
- `sayn-state` crate implementing manifest hashing (`manifest.json`), snapshots (`sayn snapshot` & `sayn rollback`), clean deployable application export (`sayn export`), and safe cache cleanup (`sayn clean`).
- Incremental compilation in `sayn craft`: hashes instruction sources with SHA-256 and reuses verified generated output when `.sayn` files are unchanged.
- Expanded CLI commands per `COMMANDS.md` specification: `sayn watch`, `sayn fix`, `sayn snapshot`, `sayn rollback`, `sayn export`, `sayn clean`, `sayn permissions`, `sayn whoami`, `sayn logout`, `sayn provider`.
- Comprehensive CLI unit tests and automated test coverage across all new subcommands.

## [0.0.4] - 2026-10-28

### Fixed
- Fixed cross-platform CRLF line ending discrepancy in formatter golden test on Windows CI runners.
- Added `.gitattributes` to enforce consistent LF line endings across all platforms for `.sayn`, `.rs`, and `.toml` files.

## [0.0.3] - 2026-10-28

### Added
- `sayn-providers` crate: `Provider` abstraction trait, deterministic `MockProvider`, and OpenAI-compatible BYOK provider with authentication checks.
- Comprehensive `install.sh` installer for Linux and macOS with automated dependency checks (Git, Rust, Node.js).
- Windows PowerShell installer (`install.ps1`) for seamless cross-platform installation.
- Release workflow publishing multi-platform binaries on every commit to `main`.
- Installed `sayn` binary into user PATH (`~/.local/bin/sayn` and `~/.cargo/bin/sayn`).

## [0.0.2] - 2026-10-28

### Added
- Formatter golden tests and Unicode idempotence verification in `sayn-formatter`.
- Path traversal and directory escape security tests in `sayn-security`.
- Scaffolding integration tests for `sayn create`, `sayn init`, and non-empty directory conflict detection in `sayn-project`.
- Repository topic tags on GitHub.

## [0.0.1] - 2026-10-28

### Added
- Initial Sayn compiler toolchain repository bootstrap.
- Core Cargo workspace layout with 7 crates:
  - `sayn-cli`: CLI binary with commands (`create`, `init`, `check`, `plan`, `craft`, `build`, `format`, `status`, `diff`, `doctor`, `auth`, `model`, `config`).
  - `sayn-core`: Domain models, diagnostics, targets, and exit codes.
  - `sayn-source`: Source discovery, `.sayn` parser, AST, and validation.
  - `sayn-formatter`: Deterministic, non-AI formatter and diff utility.
  - `sayn-project`: Project configuration (`project.toml`), scaffolding, and diagnostics.
  - `sayn-ir`: Intermediate representation and generation plan types.
  - `sayn-security`: Path confinement, safe filesystem writes, and permission policies.
- Custom commit-driven versioning automation (`scripts/version.py`, `scripts/commit-task.sh`, `.githooks/pre-commit`).
- GitHub Actions CI workflows for formatting, linting, tests, and smoke tests.
- Initial project documentation, LICENSE (MIT), README, CONTRIBUTING, and SECURITY policy.
