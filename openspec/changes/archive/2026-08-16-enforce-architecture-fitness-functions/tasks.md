## 1. Frontend and Repository Fitness Rules

- [x] 1.1 Add the stable architecture rule registry and shared actionable diagnostic formatter.
- [x] 1.2 Implement TypeScript-AST checks for React Tauri imports/invocations, native adapter access, runtime-specific branches, and prohibited state-management imports.
- [x] 1.3 Implement focused conformance guards for both `AgentService` runtime adapters without duplicating TypeScript assignability.
- [x] 1.4 Add syntax-valid positive and negative frontend fixtures and unit tests for every new detector and diagnostic location.

## 2. Native Fitness Rules

- [x] 2.1 Attach stable rule ids and repair directions to existing native dependency-direction and cross-context diagnostics.
- [x] 2.2 Complete command-thinness and bootstrap-only concrete assembly detection using the existing `syn` architecture test implementation.
- [x] 2.3 Add positive and negative native fixtures for domain, application, cross-context, command, and composition-root rules.

## 3. Unified Gate and CI

- [x] 3.1 Add `npm run architecture:check` to orchestrate focused frontend/repository checks, ESLint, TypeScript conformance, and the native architecture test target.
- [x] 3.2 Add a named architecture fitness step to CI without removing or weakening existing validation.
- [x] 3.3 Verify failure evidence by temporarily injecting direct component `invoke()`, Zustand import, and domain-to-infrastructure violations, then restore compliant fixtures/source.

## 4. Verification and Governance

- [x] 4.1 Run architecture unit tests and `npm run architecture:check`, recording repeatable command duration as performance evidence.
- [x] 4.2 Run `npm run lint:ci`, `npm run test`, `npm run build`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 4.3 Run Rust format, Clippy all-targets with warnings denied, full Rust tests, and Cargo check using the exact repository commands.
- [x] 4.4 Run strict main-spec and change validation; record UI/visual checks as not applicable because no UI behavior or visuals changed.
- [x] 4.5 Run applicable Linux desktop unit and full desktop tests; report Windows and macOS native smoke as `NOT RUN` unless actual platform evidence is available.
- [x] 4.6 Complete implementation verification against every requirement and scenario, resolve all critical findings, and mark the change ready for archive.
