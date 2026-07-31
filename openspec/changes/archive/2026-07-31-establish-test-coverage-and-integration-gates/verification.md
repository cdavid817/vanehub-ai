## Verification Summary

Verified on 2026-07-23 from `codex/engineering-reliability`.

### Coverage

| Scope | Covered lines | Total lines | Result | Gate |
| --- | ---: | ---: | ---: | ---: |
| Frontend production source | 2,796 | 6,182 | 45.23% | 45.20% |
| Native crate | 36,983 | 54,590 | 67.75% | 67.00% |
| Agent startup and terminal control | 197 | 246 | 80.08% | 80.00% |
| MCP routing | 205 | 251 | 81.67% | 80.00% |
| SQLite transactions, pool, and migrations | 824 | 937 | 87.94% | 80.00% |

Frontend V8 coverage also reported 42.37% statements, 40.03% branches, and 36.74% functions. The frontend include rule counts unimported `src/**/*.{ts,tsx}` files and retains the supported Web/mock adapters. The frontend baseline was remeasured after merging `feat: optimize frontend rendering and lazy loading (#23)`, which added production modules and tests to the denominator.

### Test and quality results

- Vitest: 84 files and 282 tests passed under V8 coverage after synchronizing with current `main`.
- Rust: 675 unit/integration tests passed and 3 child-process fixtures were ignored by the direct harness; 8 architecture tests also passed.
- Native lifecycle integration: 3 scenarios passed for success, startup compensation, and idempotent cleanup. The successful path observes the published Operations API, persisted Session transitions through `running` and `stopped`, and terminal-registry cleanup; the failure path verifies command-boundary and persisted-log redaction.
- Playwright Chromium: 52 scenarios passed against this worktree's Web runtime.
- Coverage-policy fixtures: 5 passed, including the four required negative-policy cases and cross-platform path normalization.
- ESLint with zero warnings, TypeScript/Vite build, frontend contract checks, Rust fmt/check, all-target Clippy with `-D warnings`, and both strict OpenSpec validations passed.

### CI artifacts

- `frontend-coverage-${{ github.run_id }}` retains `coverage/frontend/`, including HTML/LCOV, machine-readable totals, and `policy-summary.md`, for 14 days.
- `native-coverage-${{ github.run_id }}` retains `coverage/native/coverage.json` and the human-readable `policy-summary.md` for 14 days.
- `playwright-report-${{ github.run_id }}` retains Playwright reports and failure evidence for 14 days.

### Exclusions and deferred follow-ups

- Frontend reports exclude tests, declarations, generated output, and shared test utilities; no supported desktop or Web/mock production adapter is excluded.
- Native measurement covers compiled crate source. Critical groups are the production Agent terminal service, MCP relay, Session transaction implementation, database pool, and migration implementation; their test modules are separate source files or excluded by the coverage tool so test scaffolding does not inflate or dilute production-path percentages.
- The 3 ignored Rust tests are executable child fixtures for bounded process timeout/output and MCP stdio relay tests. Their parent tests spawn them explicitly; they are not skipped product scenarios.
- Native integration uses temporary SQLite storage, deterministic ports, and loopback-only MCP fixtures. It requires no provider CLI, network service, credential, persistent user data, or Tauri window.
- Wider frontend/native gates are measured non-regression baselines, not repository-wide 80% targets. Function and branch percentages remain reported but non-blocking until stable feasibility is measured.
- The native baseline is 67.00% against a measured Windows result of 67.75%. The Ubuntu native-coverage job independently enforces the same baseline and strict 80% critical-group requirements.
