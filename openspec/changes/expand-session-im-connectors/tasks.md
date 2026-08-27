## 1. Native Access Model

- [x] 1.1 Add a repeat-safe SQLite migration that enables matching connector access for existing non-Feishu session bindings while leaving unbound sessions disabled.
- [x] 1.2 Extend binding lookup to accept an explicit connector and return the bound connector's access when a binding exists.
- [x] 1.3 Remove the Feishu-only authorization bypass so pairing, inbound admission, and completion notifications enforce access for every connector.
- [x] 1.4 Add repository and application tests for default denial, connector isolation, legacy binding backfill, replacement, disable/re-enable, and access/admission races across all connector ids.

## 2. Typed Service Adapters

- [x] 2.1 Extend `ImService.getSessionBinding` and strict contracts with an explicit connector argument without weakening response validation.
- [x] 2.2 Update the Tauri command, mapper, and frontend adapter to pass and validate the requested stable connector id.
- [x] 2.3 Update the Web/mock adapter to persist isolated session/connector access and apply bound-connector precedence.
- [x] 2.4 Add contract and adapter tests for all connector ids, malformed responses, missing access, and cross-connector isolation.

## 3. Session Information Panel

- [x] 3.1 Add selected-connector state to `useSessionImState` with bound-connector initialization, request versioning, and stale mutation protection.
- [x] 3.2 Replace Feishu-only filtering with an accessible localized connector selector while disabling selection during binding and pending operations.
- [x] 3.3 Keep access, pairing, replacement, pause, notification, and removal operations scoped to the selected or bound connector.
- [x] 3.4 Add synchronized locale copy and hook/component tests for selection, connector isolation, restart restoration, errors, and narrow layouts.

## 4. End-to-End Verification

- [x] 4.1 Extend Playwright coverage for multiple ready connectors, selected-connector pairing, bound connector precedence, and Web/mock isolation.
- [x] 4.2 Extend deterministic native desktop fixtures and WebdriverIO coverage for non-Feishu default denial, enabled delivery, connector isolation, and relaunch persistence.
- [x] 4.3 Verify retained fixture evidence and unified logs contain no credentials, external identities, prompts, responses, or raw protocol payloads.

## 5. Repository Verification

- [x] 5.1 Run focused frontend, native communications, Playwright IM, and deterministic desktop IM tests and fix every regression.
- [x] 5.2 Run the AGENTS.md frontend, Rust, architecture, coverage, contract, and OpenSpec validation commands required by the affected areas.
- [x] 5.3 Record actual verification outcomes in this change and run `openspec validate expand-session-im-connectors --strict`.

## Verification Record

- Focused frontend connector tests: 4 files and 46 tests passed.
- Frontend coverage: 318 files and 1,689 tests passed; statements 73.09%, branches 68.63%, functions 68.70%, and lines 76.75%.
- Playwright IM settings: 4 tests passed, including Telegram selection, binding precedence, and connector isolation.
- Deterministic Windows desktop IM: 4 WebdriverIO spec files passed. Evidence: `test-results/desktop/2026-08-27T14-13-15-064Z-a2f7a4f7`.
- Desktop evidence safety: 10 retained files scanned with no findings under `feishu-evidence-safe-metadata-v1`.
- Rust workspace: 4,630 main-library tests passed with 15 ignored; 54 architecture tests and all workspace companion tests passed.
- Desktop fixture feature: the Telegram normalizer and safe-identity test passed with `desktop-e2e` enabled.
- Repository gates passed: lint, production build, formatting, workspace check, Clippy with warnings denied, native panic check, desktop unit tests, coverage policy, version tests, contracts, architecture fitness, all 139 main specs, and this strict change validation.
