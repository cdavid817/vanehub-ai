## 1. Provider Domain Contract

- [x] 1.1 Add validated `AgentProviderId`, `ProviderMetadata`, typed `ProviderCapabilities`, readiness prerequisites, and opaque `ProviderSessionRef` value types under `agent_runtime`.
- [x] 1.2 Add focused provider errors and unit tests for blank/control-character ids, invalid metadata, and inconsistent capability declarations.

## 2. Registry and Compatibility Providers

- [x] 2.1 Add an immutable provider registry with deterministic listing, exact lookup, duplicate-registration rejection, and unknown-provider errors.
- [x] 2.2 Add compatibility provider declarations for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI using the existing invocation, output-parser, terminal, and readiness collaborators.
- [x] 2.3 Add contract tests proving all five providers expose valid metadata/capabilities and that unsupported capabilities are never inferred from ids.

## 3. Runtime Composition

- [x] 3.1 Construct and inject the static provider registry in the Rust/Tauri composition root without adding a global mutable singleton.
- [x] 3.2 Route provider resolution and descriptor lookup through the registry while preserving the existing `AgentProcessGateway`, `AgentTerminalGateway`, API Agent route, Tauri commands, and frontend service contracts.
- [x] 3.3 Wrap the existing nullable Session runtime id as `ProviderSessionRef` at the Agent Runtime/Sessions gateway boundary without changing SQLite storage.

## 4. Compatibility and Architecture Guardrails

- [x] 4.1 Extend existing invocation and output fixtures to prove compatibility providers preserve arguments, prompt delivery, resume ids, parsing, and usage for all five built-in CLIs.
- [x] 4.2 Extend `src-tauri/tests/architecture.rs` to reject concrete provider imports and built-in provider-id branching from Sessions domain/application and provider-neutral Agent Runtime application modules, with narrow documented exceptions.
- [x] 4.3 Add composition tests for catalog/provider consistency, deterministic registration, duplicate ids, and unsupported provider lookup.

## 5. Verification

- [x] 5.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 5.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 5.3 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 5.4 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.5 Run `openspec validate introduce-agent-provider-contract --strict` and `openspec validate --specs --strict`.
