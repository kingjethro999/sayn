# ADR 0001: Provider generation is the MVP compiler path; the Hermes gateway is an optional backend

- **Status:** Proposed (awaiting maintainer approval)
- **Date:** 2026-11-13
- **Relates to:** `CLAUDE.md` sections 08, 09, 12 and 16; `docs/architecture/hermes-decomposition.md`

## Context

Sayn needs a model to turn `.sayn` instructions into a Next.js application. Two integration routes exist in this repository:

1. **Direct provider generation.** The Rust engine sends one bounded request to an OpenAI-compatible endpoint (Nous Portal or a BYOK provider) and receives a versioned list of file operations (`sayn-ir::GenerationResponse`). The engine validates the list, stages it, checks the candidate application and commits it as a transaction.
2. **The Hermes TUI gateway.** A Python process (`python -m tui_gateway.entry`) that runs a full agent loop and speaks newline-delimited JSON-RPC 2.0 on stdio.

The contract in `CLAUDE.md` requires that the Rust engine, not the model or an agent, decides every file write and every local command, and that Hermes stays optional and replaceable.

Reading the vendored upstream revision (`780d5a33`, `vendor/hermes-upstream/tui_gateway/`) shows why the gateway cannot yet satisfy that requirement:

- The gateway's agent uses its own tools to edit files and run shell commands inside the session's working directory. Those writes happen in the Python process. They do not pass through `sayn-security` path confinement, the manifest conflict check, or the staging transaction.
- The gateway asks its client questions through server-to-client requests (`approval`, `clarify`, `sudo`, `secret`, and others). Sayn has no mediated implementation of any of them.
- The surface is large (well over a hundred methods) and its parameter models reject unknown keys, so the client must track upstream changes.

## Decision

1. **`sayn craft` uses direct provider generation.** This is the only path that produces generated files in the MVP. Credentials, model selection, response validation, staging and validation are all owned by Rust.
2. **The Hermes gateway is an optional agent backend behind a tested boundary, and is not wired into `sayn craft`.** `sayn-hermes-runtime::HermesRpcClient` implements the transport only:
   - waits, bounded, for `gateway.ready`;
   - sends `client.capabilities {server_requests: true}` and then answers **every** gateway-initiated request with JSON-RPC error `-32601`, so the agent is refused at once instead of stalling;
   - supports `session.create`, `prompt.submit`, `session.interrupt`, event streaming, and cancellation by stopping the process;
   - bounds every wait and always reaps the child process.
3. **The gateway may become a generation backend only when a design guarantees Rust-owned writes.** Acceptable designs include running the agent against a disposable copy of the output and importing the result through `GenerationTransaction`, or disabling the agent's file and shell tools and having it return the same structured operation list a provider returns. Either needs its own ADR.

## Consequences

- Formatting, scaffolding, craft and build do not need Python or the managed Hermes runtime. `sayn doctor` reports the runtime as optional.
- Generation is a single request per craft. There is no agent loop, tool use or multi-step repair in the MVP; a response that fails validation is discarded and reported.
- Nous Portal credentials and the model catalog are shared by both routes, so adopting the gateway later does not change authentication.
- The client is tested against a fixture (`crates/sayn-hermes-runtime/tests/fixtures/fake_gateway.py`) that follows the vendored contract for the methods used. **It has not been run against a live gateway.** A compatibility test against the pinned revision is required before the gateway is used for anything.
- Upstream contract changes can break the client. The pinned revision in `docs/architecture/hermes-decomposition.md` must be updated together with the fixture.

## Alternatives considered

- **Gateway as the primary path now.** Rejected: agent tool writes bypass confinement, conflict detection and the transaction, which the product contract forbids.
- **Forking Hermes to route tool calls through Sayn.** Rejected for now: a long-lived fork is a deliberate decision that the contract says to avoid until a documented integration is shown to be insufficient.
- **Removing the Hermes runtime crate.** Rejected: Nous Portal authentication and model discovery are in use, and the gateway boundary is cheap to keep tested.
