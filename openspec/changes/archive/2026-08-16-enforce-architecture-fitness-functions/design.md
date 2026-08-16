## Context

See `proposal.md` for motivation. The repository already has strong but dispersed enforcement: ESLint owns TypeScript bans and the 300-line limit; TypeScript checks both runtime clients against `AgentService`; `src-tauri/tests/architecture.rs` parses Rust with `syn` and already covers much of dependency direction, command thinness, and bootstrap assembly; CI runs these checks only as parts of broad jobs. The missing layer is a stable registry, complete frontend detection, explicit fixture contracts, and a named unified gate.

### Existing rule matrix

| Rule | Current document | Current machine check | Gap | This change |
| --- | --- | --- | --- | --- |
| React cannot use Tauri directly | `AGENTS.md`, `frontend-runtime-architecture` | Focused Rust test scans TSX text for selected tokens | Not AST-backed; incomplete native-adapter/runtime-global detection; no stable id | Add TypeScript-AST frontend detector and fixtures |
| Tauri/Web adapters share one contract | `AGENTS.md`, `frontend-runtime-architecture` | Both clients are typed as `AgentService`; build catches shape drift | Conformance annotations can be removed without a focused diagnostic | Guard both annotations and run build/type checking through the gate |
| React built-in state/context only | `AGENTS.md`, roadmap 01 | No focused production import check; prohibited packages are currently absent | Redux/Zustand/MobX can be introduced later | Add package and production-import rules with fixtures |
| Domain/application dependencies point inward | `openspec/project.md`, `native-runtime-architecture`, ADR | `syn`-based native architecture test | Diagnostics lack stable ids; detector cases are concentrated in one file | Add rule ids and explicit positive/negative fixtures |
| Commands are thin | `openspec/project.md`, `native-runtime-architecture` | `syn` command metrics detect SQL/process/I/O and excess policy flow | Diagnostics lack stable ids and some fixture assertions | Extend fixture and diagnostic contracts |
| Bootstrap owns concrete assembly | `openspec/project.md`, ADR | Runtime-I/O detector in native architecture test | Needs explicit stable diagnostics and negative fixtures | Extend existing detector, not a second scanner |
| Cross-context access uses public API | `openspec/project.md` | `syn` dependency detector rejects non-`api` imports from domain/application | Coverage outside inward layers and fixture clarity are incomplete | Generalize reviewed context scan and fixture cases |
| TS `any`, `@ts-ignore`, max 300 lines | `AGENTS.md`, `openspec/project.md` | ESLint; documented finite legacy max-lines list | Not exposed as architecture entry point | Reuse ESLint in the unified command; do not add exemptions |
| Rust panic shortcuts | `AGENTS.md`, `openspec/project.md` | Clippy plus existing test-only use pattern | Unified gate does not invoke the configured Rust policy | Reuse the narrow native architecture target and Clippy remains in full validation |
| CI enforcement | `continuous-integration` | Broad frontend/native jobs | No named architecture gate | Add explicit `Architecture fitness` step |

## Goals / Non-Goals

**Goals:**

- Make violations deterministic, local, and actionable through stable rule ids.
- Reuse existing ESLint, TypeScript, and Rust AST enforcement rather than create competing scanners.
- Keep the fast developer command bounded enough for repeated local use.
- Prove each newly introduced detector with accepting and rejecting fixtures.

**Non-Goals:**

- Changing the frontend service API, runtime behavior, bounded-context map, or UI.
- Implementing roadmap item 02 or any later Agent platform feature.
- Removing unrelated historical max-lines exemptions or repairing unrelated working-tree changes.
- Replacing semantic architecture review with heuristics.

## Decisions

### 1. Use a small JavaScript rule registry for frontend and repository rules

`scripts/architecture/` will contain separate modules for rule definitions, TypeScript AST traversal, diagnostics, and the command entry point. The installed TypeScript compiler provides parsing, so no production or development dependency is added. A registry entry carries a stable id, category, and repair text.

Alternative: ESLint-only custom rules. Rejected for this change because a local plugin would add configuration/loading complexity for repository-wide cross-file parity checks; ESLint remains authoritative for per-file language constraints.

### 2. Keep native enforcement in the existing `syn` test target

The Rust architecture test already has parsed dependency and command detectors. It will be refactored only enough to attach stable ids, report paths/lines, and exercise dedicated fixture inputs. The unified command invokes the focused `architecture` integration-test target.

Alternative: duplicate Rust scanning in Node. Rejected because it would be less syntax-aware and create two rule implementations.

### 3. Treat adapter parity as two complementary checks

The frontend detector asserts that both `tauriAgentClient` and `webAgentClient` remain explicitly typed against `AgentService`. TypeScript compilation remains authoritative for member/type parity. This catches both a drifted implementation and removal of the conformance boundary without attempting to expand object spreads manually.

Alternative: compare object literal keys. Rejected because both clients compose sub-adapters and a key comparison would be brittle and weaker than TypeScript assignability.

### 4. Unified command orchestrates, CI names it explicitly

`npm run architecture:check` runs the focused frontend detector tests/check, ESLint, TypeScript no-emit validation, and native architecture test. CI adds a named step invoking that command; existing jobs and commands remain intact.

Alternative: create a separate CI job that repeats dependency installation and compilation. Rejected because a named step provides the required gate without increasing runner setup or fragmenting existing validation.

### 5. Diagnostics are a contract

All new diagnostics use `[ARCH-<AREA>-NNN] path:line: message Repair: ...`. Tests assert ids and locations. Native assertions aggregate violations so developers see all findings in one run.

## Risks / Trade-offs

- [AST detectors can over-classify aliases or test fixtures] → Scan production roots only, resolve import module names rather than identifier spelling where possible, and use syntax-valid fixtures.
- [The unified command may be slower because Rust tests compile] → Invoke only the architecture integration-test target; performance evidence will record repeatable wall time without a brittle millisecond CI budget.
- [Cross-context rules can reject deliberate published contracts outside `api.rs`] → Encode only the documented `api`/explicit-contract/event forms already used by the repository; any new exception requires an OpenSpec/ADR change, not an allowlist entry.
- [Existing unrelated working-tree changes may fail full validation] → Report failures with ownership evidence and do not mutate unrelated files merely to obtain a green result.

## Migration Plan

1. Land the rule registry, frontend detector, and fixture tests without changing runtime output.
2. Add stable ids and fixture coverage to the existing native architecture test.
3. Add the unified npm command and named CI step.
4. Run strict OpenSpec, architecture, full repository, coverage, contract, and applicable desktop validation.
5. Rollback is deletion of the new orchestration/detector files and restoration of the package/CI entries; no data rollback is required.
