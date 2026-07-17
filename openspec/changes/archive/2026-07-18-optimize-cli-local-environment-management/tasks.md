## 1. Contracts And Persistence

- [x] 1.1 Extend shared TypeScript and Rust CLI status models with environment, installation distribution, conflict, and lifecycle eligibility fields.
- [x] 1.2 Update Tauri and Web Agent service adapters plus contract fixtures for targeted refresh and honest unsupported Web details.
- [x] 1.3 Add additive SQLite persistence and backward-compatible cached reads for detailed CLI environment status.

## 2. Native Detection And Lifecycle Safety

- [x] 2.1 Implement bounded PATH and known-location candidate enumeration, normalization, source classification, and active-entry selection.
- [x] 2.2 Probe distinct candidates with timeouts and derive runnable, broken, version, and conflict states with unified diagnostics.
- [x] 2.3 Add all-tool and single-tool asynchronous refresh behavior that persists partial results.
- [x] 2.4 Add conservative backend lifecycle eligibility validation and multiple-installation confirmation inputs without accepting command text.
- [x] 2.5 Serialize managed CLI package mutations and refresh the affected CLI after success.

## 3. CLI Management And About UI

- [x] 3.1 Refactor CLI Management into compact service-backed environment cards with per-tool refresh, source/environment badges, health, versions, active path, and diagnostics.
- [x] 3.2 Add conflict disclosure and confirmation plus safe manual guidance for broken, non-npm, and unknown active sources.
- [x] 3.3 Preserve operation progress/logs and reflect targeted refresh versus globally serialized package mutation states.
- [x] 3.4 Add a service-backed About environment summary that links to CLI Management without duplicating lifecycle actions.
- [x] 3.5 Add synchronized zh-CN/en copy and ensure semantic-token rendering for `futuristic` and `minimal`.

## 4. Tests And Documentation

- [x] 4.1 Add Rust tests for candidate deduplication, source classification, active entry, broken/conflict derivation, lifecycle eligibility, and mutation guard behavior.
- [x] 4.2 Add frontend tests for action derivation, detailed card rendering, targeted refresh, conflict/manual states, About summary, adapter parity, and i18n parity.
- [x] 4.3 Add or update Playwright coverage for CLI Management in both registered styles and zh-CN/en at representative widths.
- [x] 4.4 Update `implementation-notes.md` with the shipped short-term design, known limitations, and prioritized optimization/extension paths.

## 5. Verification

- [x] 5.1 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 5.2 Run `cargo test`, `cargo check`, and `cargo clippy` for `src-tauri/Cargo.toml`.
- [x] 5.3 Run `openspec validate "optimize-cli-local-environment-management" --strict` and `openspec validate --specs --strict`.
