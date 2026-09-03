## Why

An external documentation audit (`vanehub-docs-refactor-bundle`, 2026-09-02) reported drift across `README.zh-CN.md`, the Simplified Chinese user guide, the Simplified Chinese developer guide, and `docs/agent-infrastructure/`. The audit was produced **without repository access** — its own `audit-index.json` records "the repository could not be cloned in this execution environment" and "no local docs/test/build commands were executed" — so every claim in it is a lead, not a fact.

Checking each lead against code, the OpenSpec main specs, the settings registry, and the i18n resources confirmed a subset and refuted another subset. The confirmed defects are not stylistic. They tell a reader to run a command that does not exist, describe a security boundary the runtime does not provide, and state a diagnostic technique that the code contradicts:

- All three READMEs instruct `npm run tauri -- dev`. `package.json` registers `tauri:dev`; there is no `tauri` script, so the documented desktop start fails outright.
- All three READMEs claim 21 native bounded contexts and both `native-contexts.md` chapters claim 24. `src-tauri/src/contexts/` holds 27. The context *tables* are already enforced against the directory by `validate-docs.mjs`; only the prose totals drifted, because nothing checks prose.
- `use-cases.md` tells a reader that "logs deliberately contain no trace identifiers … correlate by time, not by id", while `observability.md`, `persistence-and-logging.md`, and `runtime_support.rs:378-384` all record that `runId`/`traceId`/`spanId` are inserted into every log entry's context. A reader following the guide would abandon the correlation path that actually works.
- `worktree.md` states that per-worktree directories mean "no extra sandbox logic is needed … OS-level directory isolation is itself the boundary". A worktree constrains neither file access outside it, nor network, nor credentials. This is a false security boundary in a document about running autonomous agents.
- `skill-management.md` classifies Skills as "stateless, plain text, no permission system required" in the same guide that documents a Wasmtime sandbox with fuel metering, epoch interruption, trust records, integrity checks, and two-level kill switches.
- `lsp-code-intelligence.md` names two different settings entry points for one page and states both "nine read-only tools" and "four LSP tools" for the same runtime.
- `troubleshooting.md` answers "can one Agent have separate memory?" with "no — memory is a host-level shared pool; the only isolation is turning memory off", while `faq.md`, `personalization.md`, and `memory.rs`'s `eligibility()` all implement per-record scope and per-record Agent audience.

Several other audit leads were refuted and are recorded as rejected rather than silently dropped, because a future reader of this change needs to know they were checked.

## What Changes

**Phase 1 — deterministic fixes and regrouping.**

- Correct the desktop start command in all three READMEs. Parity forces this: `check-readme-parity.mjs` compares code blocks verbatim across languages, so a one-language fix fails CI.
- Replace hardcoded bounded-context totals with the generated count, and extend `validate-docs.mjs` so a prose total that disagrees with `src-tauri/src/contexts/` fails the same way a missing table row already does.
- Add a README script guard: every `npm run <script>` appearing in a README must exist in `package.json`.
- Shrink the two README guide tables to grouped entry points, inside the locale-scoped block that parity already exempts.
- Regroup both mdBook `SUMMARY.md` files by task and architecture domain without renaming or moving any file.

**Phase 2 — cross-document capability calibration.** Memory scope/audience/candidate/immutable-id, log correlation identifiers, the Worktree boundary, IM inbound scope, Skill security posture, permission projection per Agent, LSP capability surface, and local-media qualification state each get one definition, applied everywhere the old wording survives.

**Phase 3 — structure.** Agent-infrastructure boundary restatement, CLI reference relocation to `docs/reference/cli/`, and the audit's split/merge program are scoped but deliberately deferred; see `design.md` for why the first two are done here and the third is not.

## Capabilities

### New Capabilities

- `documentation-fact-integrity`: repository facts that appear in prose — command names, component totals, capability inventories — are checked against their source of truth instead of hand-maintained in parallel.

### Modified Capabilities

- `multilingual-readme`: README command examples must be executable, not merely identical across languages.
- `user-guide-documentation`: a user-facing capability description must not contradict the runtime, and a security boundary must be described by what it actually constrains.
- `native-developer-documentation`: a developer chapter must not state a component total that disagrees with the tree it documents.

## Impact

**Runtime scope: neither.** Documentation, one documentation validator, and OpenSpec artifacts. No application code, no runtime adapter, no Tauri command, no database migration.

Affected files:

- `README.md`, `README.zh-CN.md`, `README.ja.md`
- `docs/user-guide/zh-CN/src/` — `SUMMARY.md`, `core-concepts.md`, `use-cases.md`, `worktree.md`, `troubleshooting.md`, `lsp-code-intelligence.md`, `permissions.md`, `remote-and-im.md`, `local-media.md`, `index.md`
- `docs/developer-guide/zh-CN/src/` — `SUMMARY.md`, `native-contexts.md`, `runtime-boundaries.md`, `cross-session-memory.md`, `retrieval.md`, `skill-management.md`, `index.md`
- `docs/developer-guide/src/native-contexts.md`, `docs/developer-guide/src/runtime-boundaries.md` — the same two defects exist in the English guide
- `docs/agent-infrastructure/README.md`, `function-calling-architecture.md`, `agent-skills-architecture.md`
- `scripts/validate-docs.mjs`, `scripts/validate-docs.node-test.mjs` — prose-total and README-script guards

Downstream: a reader who followed `use-cases.md`'s correlation guidance or `worktree.md`'s isolation claim was given wrong information; both are corrected here. The unresolved `skill_evolution_evidence` encryption conflict is recorded in `design.md` rather than resolved, because closing it requires either an implementation or a main-spec amendment, and neither belongs in a documentation change.
