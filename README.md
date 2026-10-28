# Sayn

**An AI-native, instruction-first programming language.**

Write what your software should do in `.sayn` files. Run `sayn craft`. Sayn interprets the instructions, creates or updates a real application, validates the result, and runs it locally.

> ⚠️ **Status: Early Development** — Sayn is under active initial implementation. The architecture is being established and core features are being built incrementally. Not yet suitable for production use.

## What is Sayn?

Sayn is a programming language where **human-written instructions are the source code**. Instead of writing JavaScript, Python, or TypeScript directly, you describe what your software should do in ordinary language. Sayn's compiler interprets those instructions and generates a real, deployable application.

### How it works

```sayn
# Developer Portfolio

Create a modern developer portfolio website.

## Design
Use a clean minimalist interface.
Support dark mode and light mode.
Make the website fully responsive.

## Homepage
Include an introduction section.
Include a projects gallery.
Include a skills section.
Include a contact form.

## Requirements
Use accessible components.
Optimize for performance.
```

Then run:

```bash
sayn craft
```

Sayn will:
1. **Discover** your `.sayn` instruction files
2. **Check** them for errors, missing references, and conflicts
3. **Plan** what needs to be generated or updated
4. **Generate** a real Next.js application
5. **Validate** the generated code compiles and works
6. **Start** a local development server

### Key principles

- **Instructions are source code** — `.sayn` files are your primary source of truth
- **No chat required** — `sayn craft` is a command, not a conversation
- **Real output** — generates ordinary Next.js projects you can inspect, modify, and deploy
- **Incremental** — only regenerates what changed
- **Provider-agnostic** — works with Sayn Cloud, Nous Portal, or your own AI provider (BYOK)

## Installation

> Installation packages are not yet published. Build from source for development.

### Build from source

```bash
# Prerequisites: Rust 1.85+, Git
git clone https://github.com/kingjethro999/sayn.git
cd sayn
cargo build --release

# The binary will be at target/release/sayn
```

## Quick Start

```bash
# Create a new web project
sayn create --web portfolio

# Enter the project
cd portfolio

# Edit app/page.sayn with your instructions

# Validate your instructions
sayn check

# Preview the generation plan
sayn plan

# Generate and run the application
sayn craft

# Build for production
sayn build
```

## CLI Commands

| Command | Status | Description |
|---------|--------|-------------|
| `sayn --help` | ✅ Implemented | Show help information |
| `sayn --version` | ✅ Implemented | Show version |
| `sayn create --web` | ✅ Implemented | Create a new web project |
| `sayn check` | ✅ Implemented | Validate .sayn instructions offline |
| `sayn format` | ✅ Implemented | Format .sayn files deterministically |
| `sayn plan` | ✅ Implemented | Preview generation plan |
| `sayn craft` | ✅ Implemented | Generate application (auto check+plan) |
| `sayn build` | 🚧 Stub | Production build |
| `sayn doctor` | 🚧 Stub | Environment diagnostics |
| `sayn init` | 🚧 Stub | Initialize in existing directory |
| `sayn status` | 🚧 Stub | Show project status |
| `sayn auth` | 🚧 Stub | Authentication |
| `sayn provider` | 🚧 Stub | Provider management |
| `sayn model` | 🚧 Stub | Model management |

## Architecture

Sayn is implemented as a Rust monorepo with focused crates:

- **sayn-cli** — Command parsing, terminal UI, exit codes
- **sayn-core** — Shared domain types and coordination
- **sayn-source** — `.sayn` file discovery, parsing, and span tracking
- **sayn-project** — Project discovery, `project.toml` parsing, scaffolding
- **sayn-formatter** — Deterministic source formatting (no AI required)
- **sayn-ir** — Intermediate representation and validated operations
- **sayn-security** — Path confinement, permission enforcement

## Development

```bash
# Build
cargo build

# Test
cargo test --workspace

# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-targets -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development instructions.

## Documentation

- [AGENTS.md](AGENTS.md) — Cross-agent product specification
- [CONTRIBUTING.md](CONTRIBUTING.md) — Contribution guidelines
- [SECURITY.md](SECURITY.md) — Security policy
- [docs/adr/](docs/adr/) — Architecture decision records

## License

MIT — see [LICENSE](LICENSE) for details.
