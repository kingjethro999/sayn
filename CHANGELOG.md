# Changelog

All notable changes to the **Sayn** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and follows Sayn's commit-driven version rollover rules.

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
