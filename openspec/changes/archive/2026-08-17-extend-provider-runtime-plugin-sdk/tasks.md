## 1. Proposal Gate and Baseline

- [x] 1.1 Run `openspec validate extend-provider-runtime-plugin-sdk --strict` and do not modify business code until it passes.
- [x] 1.2 Run focused existing provider/runtime tests and record the pre-change compatibility baseline.
- [x] 1.3 Inventory provider-identity branches in provider-neutral Session, generation, usage, review, evaluation, and monitoring modules; classify adapter-owned matches separately.

## 2. Provider SDK Domain and Application Contracts

- [x] 2.1 Extend provider domain values for typed capabilities, cancellation, permission/model/reasoning declarations, version/readiness probes, health classifications, and bounded parser policy.
- [x] 2.2 Extend the `AgentProvider` application contract with launch/input translation, incremental parser construction, resume, cancellation, permission/options, usage, version, readiness, and diagnostics without exposing infrastructure types.
- [x] 2.3 Add stable provider contract/manifest/capability/parser/detection/permission error classifications and map them through the existing command-safe Agent Runtime error boundary.
- [x] 2.4 Add domain and application unit tests for invariants, unsupported capabilities, error mapping, and side-effect-free specifications.

## 3. Versioned Safe Manifest

- [x] 3.1 Implement strict schema-version-1 manifest deserialization and normalization into provider domain declarations.
- [x] 3.2 Reject unknown versions/fields, duplicate keys/ids, invalid executable basenames, inconsistent capabilities, hooks, commands, arguments, environment, scripts, URLs, paths, libraries, and entrypoints.
- [x] 3.3 Add valid, malformed, adversarial, and external-provider-disabled manifest fixtures and negative tests.
- [x] 3.4 Validate built-in manifest declarations during deterministic static registry construction and fail startup atomically on invalid or duplicate declarations.

## 4. Bounded Output Parser SDK

- [x] 4.1 Implement a reusable bounded stdout/stderr byte framer that preserves partial UTF-8 and record tails and classifies oversized/malformed input.
- [x] 4.2 Adapt structured JSON event and text-fallback parsing to emit existing token, thinking, tool, session, usage, completion, and failure without duplicate completion content.
- [x] 4.3 Add deterministic chunk-partition/property tests, partial UTF-8 tests, stdout/stderr interleave tests, oversized-record tests, and malformed protocol tests.
- [x] 4.4 Preserve the active Antigravity change's unobserved `step_update` behavior and prove no invented event shape is introduced.

## 5. Built-in Provider Adapters

- [x] 5.1 Migrate Claude Code to the complete SDK contract while pinning existing launch, resume, parser, permission, usage, version, and health behavior.
- [x] 5.2 Migrate Codex CLI to the complete SDK contract while pinning existing launch, resume, parser, permission, model/reasoning, usage, version, and health behavior.
- [x] 5.3 Migrate Gemini CLI to the complete SDK contract while pinning existing launch, resume, parser, permission, model, usage, version, and health behavior.
- [x] 5.4 Migrate OpenCode to the complete SDK contract while pinning existing launch, resume, parser, permission, model, usage, version, and health behavior.
- [x] 5.5 Migrate Antigravity CLI to the complete SDK contract while pinning currently verified behavior and leaving live-capture tasks in its active change.

## 6. Provider-neutral Runtime Integration

- [x] 6.1 Route generic Session/generation process launch, parser selection, resume, cancellation, permissions, options, usage, and diagnostics through registry resolution plus capability negotiation.
- [x] 6.2 Remove superseded provider-identity business branches from provider-neutral modules while retaining static composition and adapter-owned identity declarations.
- [x] 6.3 Prove availability/version/health checks do not deliver prompts, create sessions, or start interactive processes and use unified redacted logging for safe failures.
- [x] 6.4 Add architecture/structural tests preventing new provider identity branches in generic Session orchestration and runtime consumers.

## 7. Conformance Kit and Fixture Provider

- [x] 7.1 Implement a reusable conformance harness for registration, duplicate ids, availability, launch/input mapping, cancellation, parsing, resume, unsupported behavior, redaction, version failures, manifests, and error classification.
- [x] 7.2 Run the same mandatory conformance harness for all five built-in providers with adapter-owned fixture vectors.
- [x] 7.3 Add a test-only fixture provider through test composition and prove it requires no generic Session, usage, Tauri command, or frontend adapter changes.
- [x] 7.4 Add a fake CLI desktop integration test for streaming, opaque session capture, usage, classified failure, and bounded cancellation using the existing native test runtime.

## 8. Documentation and Compatibility

- [x] 8.1 Add `docs/provider-sdk/contract.md` covering the complete internal SDK and capability negotiation.
- [x] 8.2 Add `docs/provider-sdk/manifest.md` and `docs/provider-sdk/security-rules.md` covering schema version 1, data-only validation, and the fail-closed external-provider boundary.
- [x] 8.3 Add `docs/provider-sdk/example-provider.md` and `docs/provider-sdk/conformance-testing.md` using only the test fixture provider.
- [x] 8.4 Run documentation validation and verify Tauri command, frontend `AgentService`, Tauri adapter, Web/mock adapter, and persistence contracts remain compatible.

## 9. Security and Performance Evidence

- [x] 9.1 Run manifest injection/path/hook/unknown-field negative tests, sensitive argument/output redaction tests, permission-denial tests, and parser memory-bound tests.
- [x] 9.2 Add and run reproducible fixed-fixture parser-throughput and registry-resolution benchmarks with environment/fixture evidence and deterministic structural budget assertions.
- [x] 9.3 Confirm no external package discovery/loading authority, new dynamic execution dependency, feature-local log, direct infrastructure cross-context import, or roadmap-10+ behavior was added.

## 10. Focused and Repository Validation

- [x] 10.1 Run focused provider SDK domain, application, infrastructure, conformance, parser/property, manifest, fake-CLI, architecture, security-negative, and benchmark suites.
- [x] 10.2 Run `npm run lint:ci`, `npm run test`, `npm run build`, and `npm run test:coverage`.
- [x] 10.3 Run `npm run contracts:check`, `npm run desktop:unit:test`, and `npm run test:desktop`.
- [x] 10.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 10.5 Run `npx playwright test`; if no UI files changed, record futuristic/minimal desktop/narrow visual QA as not applicable with evidence, otherwise execute the full four-cell visual matrix.
- [x] 10.6 Run `openspec validate --specs --strict` and `openspec validate extend-provider-runtime-plugin-sdk --strict`.
- [x] 10.7 Record Linux desktop result from this host and report Windows/macOS native Desktop Smoke as `NOT RUN` unless actual platform evidence is obtained.

## 11. Verification, Archive, and Post-Archive Validation

- [x] 11.1 Use the OpenSpec verification workflow to compare every requirement/scenario and task with implementation and test evidence; resolve all critical or warning findings.
- [x] 11.2 Archive only after every preceding task and acceptance scenario passes with `openspec archive extend-provider-runtime-plugin-sdk`.
- [ ] 11.3 Run `powershell -ExecutionPolicy Bypass -File scripts/Update-OpenSpecArchiveIndex.ps1` and include the synced main specs, archive directory, and index.
- [ ] 11.4 Re-run `openspec validate --specs --strict` and strict validation applicable to the archived change state, then produce the complete implementation report.
