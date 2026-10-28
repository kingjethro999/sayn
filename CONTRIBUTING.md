# Contributing to Sayn

Thank you for your interest in contributing to Sayn! This document explains how to set up the development environment, run tests, and submit changes.

## Prerequisites

- **Rust** 1.85+ (stable toolchain) with `rustfmt`, `clippy`, and `rust-analyzer`
- **Git** 2.0+
- **Node.js** 18+ and npm (for generated application testing)
- **Python** 3.10+ (optional, for future Hermes integration)

## Getting Started

```bash
git clone https://github.com/kingjethro999/sayn.git
cd sayn
cargo build
cargo test --workspace
```

## Repository Structure

```
sayn/
├── crates/
│   ├── sayn-cli/         # CLI command parsing and terminal output
│   ├── sayn-core/        # Shared types and domain contracts
│   ├── sayn-source/      # .sayn file discovery and parsing
│   ├── sayn-project/     # Project configuration and scaffolding
│   ├── sayn-formatter/   # Deterministic source formatting
│   ├── sayn-ir/          # Intermediate representation
│   └── sayn-security/    # Path confinement and permissions
├── templates/            # Project templates
├── tests/                # Integration and end-to-end tests
├── examples/             # Example Sayn projects
└── docs/                 # Documentation and ADRs
```

## Development Workflow

### Building

```bash
cargo build                    # Debug build
cargo build --release          # Release build
```

### Testing

```bash
cargo test --workspace         # All tests
cargo test -p sayn-source      # Tests for a specific crate
cargo test --workspace -- --nocapture  # With stdout
```

### Code Quality

```bash
cargo fmt --all -- --check     # Format check
cargo clippy --workspace --all-targets -- -D warnings  # Lint
```

### Adding a New CLI Command

1. Add the command variant to `crates/sayn-cli/src/cli.rs`
2. Implement the handler in `crates/sayn-cli/src/commands/`
3. Add tests in the command module
4. Update README.md command table

### Adding Tests

- Unit tests go in the same file as the code (`#[cfg(test)]` module)
- Integration tests go in `tests/`
- Golden tests for formatter go in `tests/golden/`
- Security tests go in `tests/security/`

## Commit Convention

Use conventional commits:

```
feat(cli): add plan command
fix(source): handle empty .sayn files
test(formatter): add idempotence golden tests
docs: update CLI command table
ci: add cross-platform checks
```

## Pull Requests

- Create a feature branch from `main`
- Ensure `cargo test --workspace` passes
- Ensure `cargo fmt --all -- --check` passes
- Ensure `cargo clippy --workspace --all-targets -- -D warnings` passes
- Describe what changed and why
- Reference related issues

## Reporting Bugs

Open an issue on GitHub with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Sayn version (`sayn --version`)
- Operating system and architecture

## Code of Conduct

Be respectful and constructive. We are building something new together.
