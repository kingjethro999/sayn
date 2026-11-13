# Web MVP Provider and Generation Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a truthful web MVP in which a user can authenticate with Nous Portal or configure BYOK, select only a model available through that provider, generate a Next.js application from `.sayn` instructions, validate it, and preserve the last valid output on failure.

**Architecture:** Keep Rust in control of credentials, provider selection, structured generation responses, path validation, staging, and validation. Reuse the official Nous/Hermes OAuth and OpenAI-compatible provider surfaces without letting a remote model write directly to disk. Treat the Hermes TUI gateway as a separately testable adapter boundary; the MVP generation path uses the same provider credentials and model catalog but remains fail-closed when a real inference result is unavailable.

**Tech Stack:** Rust workspace, `reqwest`, `serde`, `clap`, Nous Portal OAuth, OpenAI-compatible chat completions, Next.js App Router, pnpm/npm.

**Spec:** `AGENTS.md`, especially sections 3, 9, 11, 14-18, 22, 23, and 29-30; `docs/architecture/hermes-decomposition.md`; official Hermes programmatic-integration and Nous Portal documentation.

## Global Constraints

- `.sayn` files remain the user-authored source of truth; generated code stays under `.sayn/generated`.
- Normal `sayn craft` is non-conversational and must never report mocked, canned, or fallback output as AI-generated success.
- Nous Portal and BYOK are first-release provider modes; provider choice must be explicit and must not silently reroute.
- Credentials never enter project files, generated code, checkpoints, logs, or error bodies.
- Remote responses are untrusted proposals; every operation is schema-validated and path-confined before materialization.
- Existing valid generated output survives authentication, provider, schema, materialization, dependency, and validation failures.
- Network access, dependency installation, and local commands remain distinct capabilities.
- Existing user changes in the dirty worktree are preserved unless a test proves they are the root cause of incorrect behavior.

## Review Focus

- Expired Nous access token with a usable refresh token refreshes once and persists rotated credentials without exposing either token.
- A successful login followed by a failed or empty model-catalog request reports unknown availability; it never fabricates entitlement from a curated list.
- A selected model that is absent from the current provider catalog blocks craft before a paid inference request.
- Invalid JSON, path traversal, duplicate targets, or unsupported operations from a model leave the current generated tree untouched.
- A generation or framework-validation failure records a failed run and preserves the previous manifest and deployable application.

---

### Task 1: Establish a Truthful, Clean Baseline

**Files:**
- Modify: `crates/sayn-cli/src/commands/craft.rs`
- Modify: `crates/sayn-cli/src/commands/status.rs`
- Modify: `crates/sayn-hermes-executor/src/runner.rs`
- Modify: `README.md`
- Test: `crates/sayn-hermes-executor/src/runner.rs`

**Interfaces:**
- Consumes: existing `HermesExecutor::execute` and CLI command contracts.
- Produces: explicit `ExecutorError::ProviderUnavailable` and no local synthesis fallback after provider failure.

- [ ] **Step 1: Write failing tests** proving missing credentials and remote inference failure return errors without creating `package.json` or a completed session.
- [ ] **Step 2: Run `cargo test -p sayn-hermes-executor`** and verify both tests fail because local synthesis currently reports success.
- [ ] **Step 3: Remove silent synthesis from the provider-backed path**, classify provider/auth errors, and make CLI/status wording reflect the actual provider rather than “Integrated Engine.”
- [ ] **Step 4: Run `cargo test -p sayn-hermes-executor`** and verify the new tests pass.
- [ ] **Step 5: Run `cargo fmt --all -- --check`** and update README status claims to match implemented behavior.

### Task 2: Nous OAuth State and Refresh

**Files:**
- Modify: `crates/sayn-hermes-auth/src/lib.rs`
- Modify: `crates/sayn-cli/src/commands/auth.rs`
- Test: `crates/sayn-hermes-auth/src/lib.rs`

**Interfaces:**
- Consumes: official Nous device-code endpoints and Hermes-compatible auth-store shape.
- Produces: `NousAuthManager::valid_access_token() -> Result<String, AuthError>`, atomic credential persistence, and typed pending/slow-down/re-authentication failures.

- [ ] **Step 1: Write failing HTTP-fixture tests** for device authorization, pending polling, `slow_down`, refresh-token rotation, rejected refresh, and redacted error display.
- [ ] **Step 2: Run `cargo test -p sayn-hermes-auth`** and verify failures identify missing transport injection and refresh behavior.
- [ ] **Step 3: Introduce an injectable endpoint/client boundary**, implement official polling/refresh semantics, validate required response fields, and save credentials atomically with restrictive Unix permissions.
- [ ] **Step 4: Add a compatibility importer** that reads an existing Hermes Nous credential without overwriting the Hermes store.
- [ ] **Step 5: Run `cargo test -p sayn-hermes-auth`** and verify all auth tests pass.

### Task 3: Entitled Model Discovery and Selection

**Files:**
- Modify: `crates/sayn-hermes-models/src/lib.rs`
- Modify: `crates/sayn-cli/src/commands/model.rs`
- Modify: `crates/sayn-cli/src/commands/craft.rs`
- Test: `crates/sayn-hermes-models/src/lib.rs`

**Interfaces:**
- Consumes: `NousAuthManager::valid_access_token()` and provider `/models` responses.
- Produces: `ModelCatalog::fetch_entitled_models(provider, refresh) -> Result<Vec<ModelInfo>, ModelError>` and `validate_selection(model_id, catalog) -> Result<ModelInfo, ModelError>`.

- [ ] **Step 1: Write failing tests** for a live catalog, empty catalog, 401, malformed response, unavailable selected model, and cache expiry.
- [ ] **Step 2: Run `cargo test -p sayn-hermes-models`** and confirm fabricated curated fallbacks make the tests fail.
- [ ] **Step 3: Parse only provider-returned models as entitled**, preserve unknown capability metadata as unknown, add bounded cache metadata, and surface provider errors accurately.
- [ ] **Step 4: Require model selection validation before craft inference** while allowing an explicitly configured BYOK model when that provider cannot enumerate models.
- [ ] **Step 5: Run `cargo test -p sayn-hermes-models -p sayn-cli`** and verify the tests pass.

### Task 4: Provider-Owned Structured Generation

**Files:**
- Modify: `crates/sayn-providers/src/provider.rs`
- Modify: `crates/sayn-providers/src/openai.rs`
- Create: `crates/sayn-providers/src/nous.rs`
- Modify: `crates/sayn-providers/src/lib.rs`
- Modify: `crates/sayn-ir/src/plan.rs`
- Modify: `crates/sayn-hermes-executor/src/runner.rs`
- Test: `crates/sayn-providers/src/openai.rs`
- Test: `crates/sayn-providers/src/nous.rs`

**Interfaces:**
- Consumes: validated provider credentials, selected model, and `GenerationTask::format_prompt()`.
- Produces: `GenerationProvider::generate(&GenerationRequest) -> Result<GenerationResponse, ProviderError>` containing a versioned list of create/modify/delete/retain operations and acceptance checks.

- [x] **Step 1: Write failing fixture tests** for Nous and BYOK request construction, non-2xx classification, fenced JSON, invalid schema, unknown operation kinds, and empty operations.
- [x] **Step 2: Run `cargo test -p sayn-providers`** and verify the desired structured interfaces are absent.
- [x] **Step 3: Define the versioned response schema in `sayn-ir`** and implement provider adapters with bounded timeouts and response-size limits.
- [x] **Step 4: Make the executor depend on the provider trait**, not environment-variable probing or direct `reqwest` calls.
- [x] **Step 5: Run `cargo test -p sayn-providers -p sayn-hermes-executor`** and verify all schema/provider tests pass.

### Task 5: Transactional Materialization and Conflict Safety

**Files:**
- Modify: `crates/sayn-executor/src/materializer.rs`
- Modify: `crates/sayn-security/src/confinement.rs`
- Modify: `crates/sayn-state/src/manifest.rs`
- Create: `crates/sayn-state/src/transaction.rs`
- Modify: `crates/sayn-state/src/lib.rs`
- Test: `crates/sayn-executor/src/materializer.rs`
- Test: `crates/sayn-security/tests/security_tests.rs`
- Test: `crates/sayn-state/src/transaction.rs`

**Interfaces:**
- Consumes: validated versioned generation operations and the previous generated manifest.
- Produces: `GenerationTransaction::stage`, `validate`, `commit`, and `abort`, plus explicit manual-edit conflicts.

- [x] **Step 1: Write failing tests** for path traversal, absolute paths, symlink escape, duplicate targets, model-requested deletion, manual-edit conflicts, mid-commit failure, and rollback.
- [x] **Step 2: Run the three owning crate test suites** and verify direct writes fail the transactional expectations.
- [x] **Step 3: Stage operations under `.sayn/staging/<run-id>`**, compare current output digests with the accepted manifest, and validate the complete candidate tree before commit.
- [x] **Step 4: Atomically swap eligible files and write the manifest only after success**; preserve the prior output and journal failure otherwise.
- [x] **Step 5: Run `cargo test -p sayn-security -p sayn-state -p sayn-executor`** and verify all transaction/security tests pass.

### Task 6: Craft Orchestration, Real Validation, and Run State

**Files:**
- Modify: `crates/sayn-cli/src/commands/craft.rs`
- Modify: `crates/sayn-cli/src/commands/build.rs`
- Modify: `crates/sayn-executor/src/dependencies.rs`
- Modify: `crates/sayn-hermes-executor/src/session.rs`
- Modify: `crates/sayn-state/src/snapshot.rs`
- Test: `crates/sayn-cli/src/main.rs`
- Test: `crates/sayn-hermes-executor/src/session.rs`
- Create: `tests/integration/web_mvp.rs`

**Interfaces:**
- Consumes: auth, entitled model catalog, generation provider, generation transaction, dependency manager, and framework validator.
- Produces: a staged craft pipeline with failed/completed/cancelled run states and nonzero exit classifications.

- [x] **Step 1: Write failing integration tests** for Nous craft, BYOK craft, no credentials, unavailable model, schema failure, validation failure, unchanged-source no-inference reuse, and preserved prior output.
- [x] **Step 2: Run the focused integration test** and verify current craft incorrectly succeeds or mutates output in the failure cases.
- [x] **Step 3: Rebuild craft as explicit stages**: discover, validate source, resolve provider, refresh auth, validate model, generate, stage, validate candidate, commit, save run, optionally serve.
- [x] **Step 4: Run actual package-manager install/build commands only under policy**, capture exact exit status, and never turn warnings into success.
- [x] **Step 5: Run `cargo test --workspace`** and verify the complete suite passes.

### Task 7: Hermes Gateway Compatibility Boundary

**Files:**
- Modify: `crates/sayn-hermes-runtime/src/lib.rs`
- Create: `crates/sayn-hermes-runtime/src/rpc.rs`
- Create: `crates/sayn-hermes-runtime/tests/fixtures/fake_gateway.py`
- Modify: `docs/architecture/hermes-decomposition.md`
- Create: `docs/adr/0001-provider-and-hermes-runtime-boundary.md`

**Interfaces:**
- Consumes: newline-delimited JSON-RPC from the pinned Hermes gateway.
- Produces: a tested `HermesRpcClient` supporting `gateway.ready`, `client.capabilities`, `session.create`, `prompt.submit`, event streaming, server-request denial, cancellation, and process cleanup.

- [x] **Step 1: Write failing fake-gateway tests** for readiness, interleaved events, matching response IDs, unsupported server requests, EOF, timeout, and child cleanup.
- [x] **Step 2: Run `cargo test -p sayn-hermes-runtime`** and verify the current blocking reader cannot satisfy the protocol cases.
- [x] **Step 3: Implement a persistent reader and event channel**, advertise supported client capabilities, deny unimplemented privileged requests with JSON-RPC `-32601`, and bound waits.
- [x] **Step 4: Record the ADR**: direct provider generation is the MVP compiler path; Hermes gateway is an optional compatible agent backend until its tool mediation can guarantee Rust-owned writes.
- [x] **Step 5: Run `cargo test -p sayn-hermes-runtime`** and verify protocol tests and process cleanup pass.

### Task 8: Release-Quality Verification and Honest Documentation

**Files:**
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `docs/superpowers/plans/initial-implementation.md`
- Modify: `.github/workflows/*` as required by verified commands.

**Interfaces:**
- Consumes: all prior tasks.
- Produces: accurate implemented/experimental status and CI gates matching local verification.

- [ ] **Step 1: Add or update command examples and failure-mode documentation** for auth, model listing/selection, BYOK, craft, build, and provider failures.
- [ ] **Step 2: Run `cargo fmt --all -- --check`** and require no diff.
- [ ] **Step 3: Run `cargo clippy --workspace --all-targets -- -D warnings`** and require zero warnings.
- [ ] **Step 4: Run `cargo test --workspace` and `python3 -m unittest discover -s tests`** and require all tests to pass.
- [ ] **Step 5: Run a clean temporary-project smoke test** covering create, format check, mocked real-provider craft, unchanged craft, and production validation.
- [ ] **Step 6: Update documentation only with observed results**, listing any remaining live-provider or cross-platform verification limitations.

## Self-Review Result

- Spec coverage: the plan covers the provider/auth/model/generation/materialization/build path required for a meaningful web MVP. Hosted Sayn accounts, quotas, public installers, API/game targets, and full offline local-model inference remain outside this repair because each is a separate product subsystem.
- Type consistency: auth feeds model discovery; model selection and credentials feed provider generation; provider operations feed the transaction; the transaction feeds craft validation and run state.
- Review focus: every listed high-risk failure has an owning failing test in Tasks 2, 3, 5, or 6.
- Proportion: tasks describe observable interfaces and tests without embedding implementation bodies.
