# Verification Report

## Scope and result

- Change: `add-hybrid-local-model-runtime`
- Roadmap scope: requirement 11 only; no requirement 12 or later capability was implemented.
- Result: all delta requirements and scenarios are implemented and covered by the appropriate Rust, Vitest, contract, Playwright, visual, or native Desktop layer.
- Findings: no unresolved critical or warning findings.

## Requirement traceability

| Capability | Verified behavior | Primary evidence |
| --- | --- | --- |
| `hybrid-local-model-runtime` | Explicit provenance-bearing Profiles; bounded loopback discovery and verification; deterministic routing; privacy admission; capability negotiation; bounded context recovery; evidence-based usage; bounded streaming; redacted operations | `provider_profile.rs`, `hybrid_routing.rs`, `local_model_discovery.rs`, `api_process_adapter.rs`, Rust negative/benchmark tests, Web service tests, Windows Desktop fake server |
| `api-agent-runtime` | Optional authentication only for explicit local/private Profiles; required cloud credentials remain enforced; immutable endpoint snapshot | application readiness tests, credential-aware registry tests, API gateway tests |
| `onepiece-native-agent` | Catalog invariants and custom Profile lifecycle; shared gateway execution; Desktop local text stream; Web/Tauri adapter parity | application/repository tests, adapter contract tests, Desktop smoke |
| `agent-context-engine` | Routed/fallback Profile budget is selected before planning and is not inherited from model name | routing/context application tests and Context Engine benchmark |
| `agent-context-measurement` | Verified/configured/unknown provenance and same-model cross-endpoint isolation | Profile/context domain and application tests |

The roadmap acceptance scenarios are covered across layers: Playwright exercises the complete settings workflow and route preview; Rust covers unsupported tools, policy fallback/waiting, context limits, missing usage, and security negatives; native Desktop E2E connects to a deterministic localhost OpenAI-compatible server, lists models, verifies metadata, saves a no-auth Profile, establishes readiness, and streams a real text turn without usage.

## Architecture decisions

- Reused the existing Agent Runtime bounded context, OnePiece Profile services, API Agent contracts, Context Engine, unified operations/logging, OpenAI-compatible gateway, and Web/Tauri adapters.
- Added no endpoint-product-specific generation runtime and no parallel frontend service boundary.
- Routing freezes one immutable Profile snapshot before context planning and request construction.
- Automatic discovery remains explicit, loopback-only, allowlisted, bounded, metadata-only, redirect-safe, and prompt-free.
- Capability and context facts retain configured/verified/unknown provenance; model names do not grant capabilities or capacity.
- `local-only` admission stops before provider contact when no compatible local route exists.

## Migration and compatibility

- SQLite schema migration 78 adds endpoint Profile metadata and ordered Hybrid Routing rules.
- Legacy identities, active Profile state, provider/model/interface values, and credential references are preserved.
- Only loopback legacy endpoints receive the conservative local classification; arbitrary endpoints are not upgraded to local or verified.
- Dangling routing references are cleaned or disabled transactionally.
- Catalog endpoint fields remain immutable; custom Profile credentials are preserved, replaced, or removed only through the existing credential boundary.

## Functional, security, and performance evidence

- `npm run lint:ci`: PASSED.
- `npm run test`: PASSED, 283 files and 1,292 tests.
- `npm run build`: PASSED; production bundle and chunk policy passed.
- `npm run test:coverage`: PASSED; statements 70.34%, branches 66.50%, functions 65.95%, lines 74.37%.
- `npm run coverage:policy:test`: PASSED, 5 tests.
- `npm run version:unit:test`: PASSED, 9 tests.
- `npm run contracts:check`: PASSED, 3 tests.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: PASSED.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: PASSED.
- `cargo test --manifest-path src-tauri/Cargo.toml`: PASSED; library 3,487 passed and 15 ignored, plus permission, architecture, and MCP suites.
- `cargo check --manifest-path src-tauri/Cargo.toml`: PASSED.
- Security negative rerun: PASSED; loopback-only allowlist, unsafe URL, malformed/oversized/redirect/timeout, no prompt leakage, unsupported tools, and local-only no-cloud fallback.
- Structural performance rerun: PASSED; bounded discovery, single-pass 10,000-rule evaluation, Context Engine operation budget, and ordered 20,000-frame streaming partition.

## UI, visual, and E2E evidence

- `npx playwright test`: PASSED, 156 tests.
- Hybrid Playwright suite: PASSED, 5 tests.
- Visual inspection: PASSED for `futuristic` and `minimal` at 1440px desktop and 390px narrow widths; no page overflow, overlap, clipping, blank panel, or inaccessible focus regression was found.
- `npm run desktop:unit:test`: PASSED, 11 tests.
- `npm run test:desktop`: PASSED on Windows x64.
- Desktop evidence: `test-results/desktop/2026-08-17T17-24-36-592Z-a3a06cd3`.
- Windows native Desktop Smoke: PASSED.
- macOS native Desktop Smoke: NOT RUN.
- Linux native Desktop Smoke: NOT RUN.

## Specification evidence

- Pre-implementation `openspec validate add-hybrid-local-model-runtime --strict`: PASSED.
- `openspec validate --specs --strict`: PASSED, 136 specifications.
- `openspec validate add-hybrid-local-model-runtime --strict`: PASSED.
- `git diff --check`: PASSED.

## Limitations and follow-up dependencies

- Native results are reported only for the Windows x64 host actually executed; macOS and Linux require their own native CI runners.
- Discovery identifies protocol shape and bounded model metadata; it does not claim local endpoints are secure or trustworthy.
- Provider-specific capability self-description remains conservative where the endpoint exposes no reviewed metadata.
- Roadmap requirement 12 and later items were not implemented; they remain separate future changes.
