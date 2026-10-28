# Sayn Initial Implementation Plan

## Status: In Progress

## Environment
- **OS**: Ubuntu Linux x86_64
- **Rust**: 1.99.1 (stable)
- **Node.js**: v24.22.1
- **GitHub CLI**: Authenticated as `kingjethro999`
- **Git identity**: kingjethro999 / kingjethro999@users.noreply.github.com

## Milestone 1: Repository and Rust Foundation ⬜
- [x] Environment verification
- [x] Read AGENTS.md, CLAUDE.md, COMMANDS.md
- [ ] Initialize Git repository with `main` branch
- [ ] Create Cargo workspace with initial crates (sayn-cli, sayn-core, sayn-source, sayn-project)
- [ ] Implement CLI with clap: --help, --version, and all subcommand stubs
- [ ] Add unit tests for CLI parsing
- [ ] Create .gitignore, LICENSE (MIT), README.md, CONTRIBUTING.md, SECURITY.md
- [ ] Create rust-toolchain.toml, .editorconfig
- [ ] Create GitHub Actions CI workflow
- [ ] Run cargo build, cargo test, cargo fmt --check, cargo clippy
- [ ] Create GitHub repository via `gh repo create`
- [ ] Commit and push verified foundation
- [ ] Verify CI execution

## Milestone 2: Instruction Language Foundation ⬜
- [ ] Implement project.toml parsing (sayn-project)
- [ ] Implement project root discovery
- [ ] Implement .sayn source file discovery
- [ ] Implement .sayn parser with sections, headings, code blocks, references
- [ ] Implement source span tracking
- [ ] Implement deterministic formatter
- [ ] Implement `sayn check` (offline validation)
- [ ] Implement `sayn format`
- [ ] Add golden tests for formatter
- [ ] Add security tests for path traversal
- [ ] Add unit tests for parser

## Milestone 3: Project Scaffolding ⬜
- [ ] Implement `sayn create --web` with directory handling
- [ ] Create web-next template with example .sayn files
- [ ] Implement conflict detection for existing directories
- [ ] Implement `sayn init` for existing projects
- [ ] Implement `sayn doctor` environment checks
- [ ] Add template registry foundation
- [ ] Add scaffolding integration tests

## Milestone 4: Planning and Generation Engine ⬜
- [ ] Define IR types for generation plan
- [ ] Implement `sayn plan` command
- [ ] Implement provider abstraction trait
- [ ] Implement mock provider for tests
- [ ] Implement BYOK OpenAI-compatible provider
- [ ] Implement `sayn craft` with auto check+plan
- [ ] Implement file operation validation
- [ ] Implement Next.js target adapter
- [ ] Implement generation state/checkpoints
- [ ] Add end-to-end tests with mock provider

## Milestone 5: Development Runtime ⬜
- [ ] Implement dev server orchestration
- [ ] Implement `sayn build` with real Next.js build
- [ ] Implement execution checkpoints and resume
- [ ] Implement `sayn status`
- [ ] Add incremental regeneration

## Implementation Order
Execute milestones sequentially. Each must compile and pass tests before proceeding.
