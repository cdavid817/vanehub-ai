## 1. Baseline and evidence

- [x] 1.1 Add focused failing frontend tests for continuous chat framing, minimal first-use defaults, Zhipu icon resolution, custom endpoint selection, and the compact add-profile flow.
- [x] 1.2 Record official CLI parameter source URLs, review dates, and supported safe candidates for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI.
- [x] 1.3 Add catalog contract tests that reject policy-owned, secret, prompt, session, structured-output, and unverified flags.
- [ ] 1.4 Add native regression fixtures reproducing legacy built-in Skill registry/cache drift for the four reported Skill ids and proving the pre-fix sync does not converge.

## 2. Workspace and theme polish

- [x] 2.1 Remove the duplicate outer composer card/shadow treatment and integrate runner controls, transcript, and composer under one workspace-owned surface.
- [x] 2.2 Change shared desktop and Web/mock first-use and invalid-value theme defaults to `minimal` without overwriting valid persisted choices.
- [x] 2.3 Update focused component/settings tests and synchronized locale resources affected by the workspace and theme behavior.

## 3. Agent configuration dialog

- [x] 3.1 Centralize provider icon alias and asset fallback handling so Zhipu GLM renders reliably in provider choices and selected summaries.
- [x] 3.2 Add the stable custom endpoint/provider catalog entry and route preset and custom profile saves through the existing Agent service contracts in desktop and Web/mock modes.
- [x] 3.3 Refactor the add-profile dialog into compact provider-selection and grouped form regions with responsive scrolling, sticky actions, keyboard flow, and semantic tokens.
- [ ] 3.4 Add interaction, accessibility, narrow-viewport, and locale parity tests for preset and custom-provider flows.

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
- [ ] 5.4 Add application, filesystem, SQLite transaction, failure-injection, Web/mock parity, and UI regression tests for full and partial convergence.

## 6. Verification

- [x] 6.1 Run focused Vitest and Rust tests while implementing and resolve all regressions.
- [ ] 6.2 Run `npx playwright test` and visually inspect representative workspace and Agent Configuration states in minimal and futuristic themes at desktop and narrow widths.
- [x] 6.3 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 6.4 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 6.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.6 Run `openspec validate polish-agent-settings-cli-and-skill-sync --strict` and `openspec validate --specs --strict`, then record implementation verification evidence in this change.

## Verification evidence

- Frontend: 287 Vitest files and 1,314 tests passed; coverage, policy, version, contracts, lint, and production build passed.
- Native: 3,553 tests passed with 15 fixture-only tests ignored; 42 architecture tests and all MCP integration tests passed. Formatting, Clippy with warnings denied, and `cargo check` passed.
- E2E: the initial full run passed 154 of 156 tests. After updating the intentional default-theme expectation, both affected suites passed 4 of 4 in isolation, including the unrelated IM timeout that had flaked under full-suite resource contention.
- OpenSpec: the change-specific strict validation and the main-spec strict validation passed.
