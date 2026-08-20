## 1. Baseline and evidence

- [x] 1.1 Add focused failing frontend tests for continuous chat framing, minimal first-use defaults, Zhipu icon resolution, custom endpoint selection, and the compact add-profile flow.
- [x] 1.2 Record official CLI parameter source URLs, review dates, and supported safe candidates for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI.
- [x] 1.3 Add catalog contract tests that reject policy-owned, secret, prompt, session, structured-output, and unverified flags.
- [x] 1.4 Add native regression fixtures reproducing legacy built-in Skill registry/cache drift for the four reported Skill ids and proving the pre-fix sync does not converge.

## 2. Workspace and theme polish

- [x] 2.1 Remove the duplicate outer composer card/shadow treatment and integrate runner controls, transcript, and composer under one workspace-owned surface.
- [x] 2.2 Change shared desktop and Web/mock first-use and invalid-value theme defaults to `minimal` without overwriting valid persisted choices.
- [x] 2.3 Update focused component/settings tests and synchronized locale resources affected by the workspace and theme behavior.

## 3. Agent configuration experience

- [x] 3.1 Centralize provider icon alias and asset fallback handling so Zhipu GLM renders reliably in provider choices and selected summaries.
- [x] 3.2 Add the stable custom endpoint/provider catalog entry and route preset and custom profile saves through the existing Agent service contracts in desktop and Web/mock modes.
- [x] 3.3 Refactor the add-profile dialog into compact provider-selection and grouped form regions with responsive scrolling, sticky actions, keyboard flow, and semantic tokens.
- [x] 3.4 Add interaction, accessibility, narrow-viewport, and locale parity tests for preset and custom-provider flows.
- [x] 3.5 Move LSP configuration, workspace trust, server testing, and runtime status to a dedicated Code Intelligence settings page with stable navigation, localization, and existing query/service contracts.
- [x] 3.6 Replace the six-item Agent tab strip with a responsive grouped selector for managed CLI Agents and OnePiece while preserving navigation-target selection and per-Agent data isolation.
- [x] 3.7 Simplify saved profile cards so primary identity/status and Apply stay visible while secondary metadata and edit/duplicate/delete actions use accessible disclosure controls.
- [x] 3.8 Split new-profile creation into provider-selection and configuration stages, open edits directly in configuration, and move optional provider-specific fields into a collapsed advanced section.
- [x] 3.9 Split OnePiece provider profiles, local runtime, and tool readiness into explicit secondary views with provider profiles selected by default.
- [x] 3.10 Add navigation-order, interaction, keyboard, accessibility, responsive, theme, and locale regression coverage for the decomposed Agent Configuration and Code Intelligence surfaces.
- [x] 3.11 Add WebdriverIO coverage against the built Tauri desktop client for grouped Agent navigation, staged provider configuration, OnePiece secondary views, and the dedicated Code Intelligence page; Web/Playwright results SHALL NOT substitute for this desktop acceptance coverage.

## 4. CLI parameter catalog expansion

- [x] 4.1 Expand the shared typed catalog and evidence manifest with officially confirmed safe parameters for every managed CLI.
- [x] 4.2 Update native authoritative validation, persistence normalization, provider argument builders, and reserved-argument checks for the expanded definitions.
- [x] 4.3 Keep Web/mock profiles, safe previews, fixtures, and frontend/native contract checks identical to desktop behavior.
- [x] 4.4 Group expanded controls into a compact responsive presentation and add complete localized labels, descriptions, value help, and risk states.
- [x] 4.5 Add provider-specific fresh, resume, and interactive argv tests proving correct order, scope, precedence, and exclusion of runtime/policy-owned flags.

## 5. Skill synchronization convergence

- [x] 5.1 Reconcile legacy built-in registry hashes and derived-cache revisions to current immutable package witnesses without overwriting mutable user/imported Skills.
- [x] 5.2 Derive and atomically persist the post-repair drift report while retaining the original report in synchronization audit output.
- [x] 5.3 Return bounded per-Skill failures for unrepaired items and refresh the Skill Management banner from the post-sync overview.
- [x] 5.4 Add application, filesystem, SQLite transaction, failure-injection, Web/mock parity, and UI regression tests for full and partial convergence.

## 6. Verification

- [x] 6.1 Run focused Vitest and Rust tests while implementing and resolve all regressions.
- [x] 6.2 Run `npm run test:desktop` through WebdriverIO against the real Tauri desktop client and inspect representative Agent Configuration and Code Intelligence states in minimal and futuristic themes at desktop and narrow widths; do not use Web-mode Playwright results as desktop acceptance evidence.
- [x] 6.3 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 6.4 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 6.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.6 Run `openspec validate polish-agent-settings-cli-and-skill-sync --strict` and `openspec validate --specs --strict`, then record implementation verification evidence in this change.

## Verification evidence

- Frontend: 287 Vitest files and 1,314 tests passed; coverage, policy, version, contracts, lint, and production build passed.
- Native: 3,555 tests passed with 15 fixture-only tests ignored; 42 architecture tests and all MCP integration tests passed. Formatting, Clippy with warnings denied, and `cargo check` passed.
- E2E: the initial full run passed 154 of 156 tests. After updating the intentional default-theme expectation, both affected suites passed 4 of 4 in isolation, including the unrelated IM timeout that had flaked under full-suite resource contention.
- OpenSpec: the change-specific strict validation and the main-spec strict validation passed.
- Agent configuration decomposition: 39 focused Vitest assertions and the complete 156-scenario Chromium E2E suite passed for grouped navigation, staged profile creation, narrow-width layout, Code Intelligence ownership, theme coverage, and locale parity. The rendered Agent Configuration screenshot was inspected; in-app Browser interaction remained unavailable because no browser instance was connected.
- Desktop acceptance: the focused WebdriverIO specification passed all 3 scenarios against the Windows x64 Tauri `desktop-e2e` artifact. It exercised grouped Agent navigation, staged provider creation, OnePiece provider/runtime/tool views, the independent Code Intelligence page, both minimal and futuristic themes, and supported narrow-window overflow checks. Four real-client screenshots were inspected. Web/Playwright results were not used as desktop acceptance evidence.
- Desktop full-suite status: `npm run test:desktop` passed all 14 WebdriverIO spec files against the freshly built Windows x64 Tauri artifact in 12 minutes 36 seconds. The reporter recorded 58 passing cases and 14 explicitly skipped/BLOCKED external capabilities; Claude HTTP 403 and Gemini browser-authentication prompts are classified as external authentication blockers rather than product replies. The runtime exited cleanly without forced process cleanup. Evidence is stored under `test-results/desktop/2026-08-20T14-46-13-401Z-0ef2ee5a`.
- Skill convergence matrix: a real SQLite and managed-filesystem fixture reproduced `metadata-changed` drift for `api-doc-generation`, `code-review`, `code-security-scan`, and `readme-generation`; all four disappeared after synchronization and a repeated synchronization was idempotent. Focused application tests proved that partial failures remain in the committed post-repair snapshot, and an injected SQLite snapshot-write failure rolled back both record and tombstone mutations. Web/mock parity and Skill UI tests passed, including refreshed post-sync state and bounded per-Skill failure reasons. The 3 focused Rust cases and 101 focused Vitest cases passed.
