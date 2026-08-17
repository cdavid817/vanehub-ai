## Why

VaneHub can verify its own software, but it cannot yet compare how reliably, efficiently, and safely OnePiece and managed CLI Agents solve the same coding task. A local, deterministic evaluation platform is needed now that the unified Context Engine and canonical Agent Run lifecycle provide stable evidence and execution identities.

## What Changes

- Add versioned benchmark task manifests with bounded fixtures, acceptance commands, static assertions, diff rules, metric collection policy, and explicit Runner/Sandbox admission.
- Add isolated benchmark and arena execution for OnePiece, managed CLI Agents, and deterministic fake Agents, with a clean fixture copy per attempt and canonical Run correlation.
- Add deterministic verification, failure classification, metric aggregation, optional versioned judge evidence, and transparent versioned comparison rules that never override failed deterministic checks.
- Persist bounded evaluation metadata and artifact references in SQLite while keeping large logs/diffs in existing bounded artifact and unified-log paths.
- Add shared frontend service contracts plus matching Tauri and Web/mock adapters for catalog, configuration, lifecycle, results, comparison, detail, timeline, and JSON export.
- Add a compact Eval/Benchmark workspace page with local catalog, run configuration, live status, result comparison, verification/diff/context/tool inspection, and export.
- Ship 3–5 deterministic fixture tasks and a fake-Agent path suitable for CI without paid models or network access.
- Preserve unavailable measurements as unavailable or estimated-with-provenance; calculate cost only from an explicit reliable pricing snapshot.

## Capabilities

### New Capabilities

- `agent-evaluation`: Versioned benchmark manifests, isolated execution, deterministic judging, metrics, persistence, arena comparison, service/UI contracts, export, privacy, and retention.

### Modified Capabilities

None. The new capability consumes the published contracts of canonical Agent Runs, Context Engine evidence, execution observability, workspace isolation, usage accounting, artifacts, and unified logging without changing their existing requirements.

## Impact

- Both desktop and Web/mock runtimes gain the same evaluation service contract; React remains isolated from Tauri APIs.
- Native implementation extends existing bounded contexts rather than introducing a parallel evaluation context: `operations` owns canonical run lifecycle, `agent_runtime` owns Agent invocation, `workspaces` owns bounded fixture isolation, and `execution_observability` owns evaluation orchestration/read models and SQLite persistence.
- Tauri commands, bootstrap assembly, SQLite migrations, frontend navigation, i18n resources, and contract/architecture tests are affected.
- No external model, public leaderboard, cloud service, or roadmap 06+ behavior is introduced.
