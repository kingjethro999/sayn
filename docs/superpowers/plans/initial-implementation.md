# Sayn Initial Implementation Plan

## Status: In Progress

## Environment
- **OS**: Ubuntu Linux x86_64
- **Rust**: 1.99.1 (stable)
- **Node.js**: v24.22.1
- **GitHub CLI**: Authenticated as `kingjethro999`
- **Git identity**: kingjethro999 / kingjethro999@users.noreply.github.com

## Milestone 1: Repository and Rust Foundation ✅
- [x] Environment verification
- [x] Read AGENTS.md, CLAUDE.md, COMMANDS.md
- [x] Initialize Git repository with `main` branch
- [x] Create Cargo workspace with initial crates (sayn-cli, sayn-core, sayn-source, sayn-project, sayn-formatter, sayn-ir, sayn-security)
- [x] Implement CLI with clap: --help, --version, and all subcommand stubs
- [x] Add unit tests for CLI parsing
- [x] Create .gitignore, LICENSE (MIT), README.md, CONTRIBUTING.md, SECURITY.md
- [x] Create rust-toolchain.toml, .editorconfig
- [x] Create GitHub Actions CI workflow
- [x] Run cargo build, cargo test, cargo fmt --check, cargo clippy
- [x] Create GitHub repository via `gh repo create`
- [x] Commit and push verified foundation
- [x] Verify CI execution

## Milestone 2: Instruction Language Foundation ✅
- [x] Implement project.toml parsing (sayn-project)
- [x] Implement project root discovery
- [x] Implement .sayn source file discovery
- [x] Implement .sayn parser with sections, headings, code blocks, references
- [x] Implement source span tracking
- [x] Implement deterministic formatter
- [x] Implement `sayn check` (offline validation)
- [x] Implement `sayn format`
- [x] Add golden tests for formatter
- [x] Add security tests for path traversal
- [x] Add unit tests for parser

## Milestone 3: Project Scaffolding ✅
- [x] Implement `sayn create --web` with directory handling
- [x] Create web-next template with example .sayn files
- [x] Implement conflict detection for existing directories
- [x] Implement `sayn init` for existing projects
- [x] Implement `sayn doctor` environment checks
- [x] Add template registry foundation
- [x] Add scaffolding integration tests

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
