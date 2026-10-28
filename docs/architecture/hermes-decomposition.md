# Hermes Agent Architectural Decomposition & Integration Specification

> Authoritative architectural audit of upstream `NousResearch/hermes-agent` for deep integration into the Sayn programming language and compilation engine.
>
> Specification Version: 2.0
> Date: 2026-10-28
> Status: Milestone 1 Deliverable (Approved Architecture)

---

## 1. Upstream Revision & Source Provenance

- **Repository**: `https://github.com/NousResearch/hermes-agent`
- **Tracked Upstream Revision**: Commit SHA `780d5a3313d7e1f64c0a0f3ec6b644d50f156e90`
- **Upstream License**: MIT License (Copyright (c) 2025 Nous Research)
- **Local Analysis Checkout**: `vendor/hermes-upstream/` (git-ignored, verified clean checkout)
- **Runtime Target**: Python `>=3.11, <3.15` (tested on host CPython 3.12.3)
- **Core Dependencies**: `httpx`, `ruamel.yaml`, `rich`, `tenacity`, `python-dotenv`, `openai`

---

## 2. Executive Summary & Core Mandate

Sayn is an AI-native, instruction-first programming language. User instructions authored in `.sayn` files are interpreted to generate, validate, and maintain real applications (initially Next.js web projects).

**Hermes Agent is NOT an optional auxiliary provider or external chat CLI.** It is the primary intelligence, multi-step task execution, and agentic reasoning engine of Sayn.

### Architectural Invariant
1. **Sayn (Rust CLI & Workspace Engine)** owns:
   - Instruction discovery and `.sayn` grammar parsing
   - Deterministic formatting (`sayn format`)
   - Pre-compilation static checks (`sayn check`)
   - High-level compilation planning (`sayn plan`)
   - Filesystem confinement and safe atomic patching (`sayn-security`, `sayn-state`)
   - Package management orchestration (`pnpm`, `npm` via `sayn-executor`)
   - Development server lifecycle and app execution (`sayn app <script>`, `sayn craft`)
   - Production build validation (`sayn build`)
2. **Hermes (Integrated Python Runtime)** owns:
   - Multi-step reasoning over ambiguous instruction specifications
   - Agent loop orchestration and bounded repair of compilation diagnostics
   - Official Nous Portal OAuth authentication (Device-Code Flow)
   - Account-level model discovery, free-tier filtering, and model switching
   - Tool calling and code synthesis governed by Sayn operations
3. **No External Hermes Prerequisite**:
   - The user installs Sayn. Sayn bootstraps and supervises its embedded runtime in `~/.sayn/runtime/venv`.
   - The user never installs `hermes` CLI globally, never opens a separate chat session, and never configures raw API keys manually when authenticating through Nous.

---

## 3. Upstream Component Inventory & Decomposition

### 3.1 Module Inventory

| Upstream Module | Primary Responsibility | Reusability in Sayn |
|---|---|---|
| `hermes_cli/auth_nous.py` | Nous Portal device code login, token refresh, shared store mirroring, JWT selection | **Direct / Core Adapt** — Authentic Nous Portal OAuth |
| `hermes_cli/auth_device_flow.py` | RFC 8628 device authorization request, polling loop, browser launcher | **Direct / Core Adapt** — Headless & GUI device code flows |
| `hermes_cli/auth_constants.py` | Endpoint URLs, client IDs (`hermes-cli`), scopes (`inference:invoke`), token skew rules | **Direct / Core Preserve** |
| `hermes_cli/models.py` | Live model discovery, catalog caching, reasoning effort, pricing tables | **Direct / Core Adapt** — Model list and metadata |
| `hermes_cli/anon_auth.py` | Free-tier detection (`guest-model`, `welcome-claim`), rate limit metadata | **Direct / Core Adapt** — Accurate free-tier filtering |
| `tui_gateway/server.py` | Central JSON-RPC 2.0 dispatch server, session management, event publishing | **Reference / Protocol Core** — Stdio RPC protocol contract |
| `tui_gateway/entry.py` | Stdio reader/writer loop, signal management, RPC handshake (`gateway.ready`) | **Direct / Core Runtime** — Clean sub-process RPC server |
| `tui_gateway/methods_session.py` | `session.create`, `session.resume`, `session.history`, `session.status` | **Protocol Core** — Persistent project execution sessions |
| `tui_gateway/methods_prompt.py` | `prompt.submit`, streaming turn tokens, agent loop step dispatch | **Protocol Core** — Instruction generation requests |
| `agent/conversation_loop.py` | Core model inference loop, tool execution dispatch, interruption handler | **Preserve in Runtime** |
| `tools/` | Terminal tools, file edit tools, web research tools | **Sandboxed under Sayn** — Constrained to `.sayn/generated` |
| `ui-tui/`, `apps/desktop/` | React/Ink terminal UI, Electron desktop app | **EXCLUDE** — Unrelated to Sayn CLI workflow |
| `gateway/platforms/` | Discord, Telegram, Slack platform bridges | **EXCLUDE** — Sayn does not run chatbot bots |

---

## 4. Authentication Architecture: Nous Portal OAuth

### 4.1 Official Flow Specification (RFC 8628)
Hermes authenticates users via the official Nous Portal Device Authorization Grant:
1. **Device Code Request**:
   - `POST https://portal.nousresearch.com/api/oauth/device/code`
   - Headers: `Content-Type: application/x-www-form-urlencoded`, `Accept: application/json`
   - Body: `client_id=hermes-cli&scope=inference:invoke`
   - Response:
     ```json
     {
       "device_code": "<secret_device_code>",
       "user_code": "WDJB-MNPQ",
       "verification_uri": "https://portal.nousresearch.com/device",
       "verification_uri_complete": "https://portal.nousresearch.com/device?user_code=WDJB-MNPQ",
       "expires_in": 300,
       "interval": 5
     }
     ```
2. **User Authorization**:
   - Sayn opens the default web browser to `verification_uri_complete`.
   - In headless / remote SSH environments, Sayn clearly displays the link and code for manual entry.
3. **Token Polling Loop**:
   - `POST https://portal.nousresearch.com/api/oauth/device/token`
   - Body: `grant_type=urn:ietf:params:oauth:grant-type:device_code&client_id=hermes-cli&device_code=<secret_device_code>`
   - Polls at `max(1, interval)` seconds.
   - Status `400` with `{"error": "authorization_pending"}` indicates user has not yet clicked approve.
   - Status `200` returns the authorized credentials:
     ```json
     {
       "access_token": "<jwt_inference_token>",
       "refresh_token": "<jwt_refresh_token>",
       "token_type": "Bearer",
       "expires_in": 3600,
       "scope": "inference:invoke",
       "inference_base_url": "https://inference-api.nousresearch.com/v1"
     }
     ```
4. **Token Refresh**:
   - `POST https://portal.nousresearch.com/api/oauth/token`
   - Body: `grant_type=refresh_token&client_id=hermes-cli&refresh_token=<token>`
   - Executed automatically when access token TTL drops below 120 seconds.

### 4.2 Credential Storage & Isolation
- Credentials stored in restricted file (`0600` permissions) at `~/.sayn/auth.json`.
- Compatible mirroring to `~/.hermes/auth.json` when requested or detected, avoiding duplicated logins for developers already using Hermes.
- Secrets are never committed, never written to project repositories, and never exposed to generated application source code.

---

## 5. Model Discovery & Entitlement Rules

### 5.1 Account-Level Eligibility
A valid login does **not** grant unrestricted access to every upstream model. Sayn categorizes models strictly by account status:
1. **Free / Guest Tier**:
   - Identified by claim flags or free tier inference endpoints (`welcome-api.nousresearch.com`).
   - Filtered via `sayn model list --free`.
   - Clearly flags token rate limits and eligible model identifiers (e.g., Nous curated reasoning models).
2. **Paid / Subscription Tier**:
   - Resolved dynamically from `https://inference-api.nousresearch.com/v1/models` using the authenticated Bearer JWT.
   - Displays model capability (context length, tool support, cost indicators).
3. **Model Selection**:
   - Persisted in Sayn project configuration or global user config (`~/.sayn/config.toml`).
   - Passed to the Hermes runtime session initialization (`session.create` / model parameter).

---

## 6. Execution Pipeline & Transport

### 6.1 Process Supervision Architecture

```text
+--------------------------------------------------------------+
|                       Sayn CLI (Rust)                        |
|  - .sayn source parsing                                      |
|  - sayn check (ast validation)                               |
|  - sayn plan (intermediate representation)                   |
|  - Security & Path Confinement (sayn-security)               |
|  - Materialization & Package Management (pnpm / next.js)     |
+------------------------------+-------------------------------+
                               |
                   Stdio JSON-RPC 2.0 Pipe
                               |
+------------------------------v-------------------------------+
|             Managed Hermes Runtime (Python 3.12)             |
|                  ~/.sayn/runtime/venv                        |
|  - tui_gateway.entry / server.py                             |
|  - Session Lifecycle (session.create / resume)               |
|  - Instruction Reasoning & Code Synthesis                    |
|  - Bounded Repair Loop                                       |
|  - Tool calling with sandboxed writes                        |
+--------------------------------------------------------------+
```

### 6.2 Stdio JSON-RPC 2.0 Protocol Handshake
1. **Startup**: Sayn spawns `python3 -m tui_gateway.entry` with redirected stdin/stdout.
2. **Handshake**: Gateway emits `gateway.ready` event:
   ```json
   {"jsonrpc": "2.0", "method": "event", "params": {"type": "gateway.ready", "payload": {"change_events": true}}}
   ```
3. **Session Creation**:
   ```json
   {"jsonrpc": "2.0", "id": 1, "method": "session.create", "params": {"workdir": "<project_dir>", "model": "<model_id>"}}
   ```
4. **Prompt Submission**:
   Sayn submits the synthesized compilation prompt containing parsed `.sayn` instructions and constraints:
   ```json
   {"jsonrpc": "2.0", "id": 2, "method": "prompt.submit", "params": {"session_id": "<sid>", "text": "<structured_instruction_task>"}}
   ```
5. **Streaming Events**:
   Gateway returns real-time progress events (`token`, `tool_call`, `file_diff`).
6. **Completion**:
   Sayn captures modified files, validates them through `sayn_security`, materializes them under `.sayn/generated`, runs package installation (`pnpm install`), and launches development verification.

---

## 7. Packaging & Distribution Plan

1. **Virtual Environment Isolation**:
   - Runtime lives in `~/.sayn/runtime/venv`.
   - Never pollutes system Python (`/usr/lib/python*`) and never requires `sudo`.
2. **Pre-flight Health Diagnostics (`sayn doctor`)**:
   - Check Rust CLI binary and version
   - Check Python 3.11+ availability
   - Check `~/.sayn/runtime` virtual environment integrity
   - Check Hermes runtime package dependencies
   - Check pnpm package manager availability
   - Check Nous Portal authentication status and active model
3. **Automatic Bootstrap**:
   - If `~/.sayn/runtime` is uninitialized, running `sayn craft` or `sayn auth` automatically provisions the environment with user-friendly progress reporting.

---

## 8. Licensing Compliance

- `NousResearch/hermes-agent` is licensed under the MIT License (Copyright (c) 2025 Nous Research).
- Reused modules retain original copyright headers and license references.
- Documentation and attribution clearly note that Sayn embeds components of the Hermes Agent runtime under the MIT license, and is an independent instruction-first programming language toolchain.
