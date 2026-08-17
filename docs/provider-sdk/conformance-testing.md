# Provider Conformance Testing

The reusable Rust conformance suite runs the same mandatory checks against Claude Code, Codex CLI, Gemini CLI, OpenCode, Antigravity CLI and the test-only fixture provider. Provider-specific expected arguments and output samples remain adapter-owned fixtures.

The suite covers deterministic registration, duplicate ids, manifest agreement, side-effect-free readiness/version specifications, launch and prompt delivery, resume ownership, cancellation bounds, permission/options, capability rejection, chunk-independent parsing, stdout/stderr separation, opaque session capture, usage, classified provider/protocol failures, and safe diagnostics.

Run focused evidence with:

```bash
cargo test --manifest-path src-tauri/Cargo.toml provider -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml provider_sdk_fixed_fixture_benchmark -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test architecture provider_neutral_layers_do_not_select_concrete_cli_providers
```

Timing output is observational and includes fixture size and environment context. Correctness enforces buffer and registry operation bounds structurally, independently of wall-clock timing. Complete changes must also pass the repository gates in `AGENTS.md`.
