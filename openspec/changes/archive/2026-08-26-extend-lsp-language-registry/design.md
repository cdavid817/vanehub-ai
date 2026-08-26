## Context

See `proposal.md` — Why. The relevant current-state facts that shape the approach:

- `LanguageFamily` and `ServerKind` are two-variant enums matched in 15 Rust files across the `code_intelligence` context and its command layer.
- `lsp_language_configurations.language_id` carries `CHECK (language_id IN ('rust', 'typescript_javascript'))`. SQLite cannot alter a `CHECK` constraint.
- `LanguageFamily::startup_arguments()` returns `&'static [&'static str]`. The `startup_arguments_json` column exists but only mirrors that constant; no command path can change it.
- `src/types/lsp.ts` exports `lspLanguageIds` as a tuple literal type, so `LspLanguageId` is a closed union in TypeScript too.
- `contexts/tooling/cli` already solves the same problem for CLI tools: `CLI_TOOL_DEFINITIONS: &[CliToolDefinition]` looked up by a validated string-newtype id (`CliToolId`), with per-tool executable names, distributions, probes, and platform policy. That pattern is proven in this codebase and is the reference for this change.

**Runtime boundary.** The registry lives in the Rust/native layer. The frontend learns the language set only through the existing `agent-service` boundary; `invoke()` stays in `tauri-agent-client.ts`, and `web-agent-client.ts` / `web-lsp-client.ts` return the same shape deterministically. No React component gains knowledge of a specific language.

## Goals / Non-Goals

**Goals:**

- One place to add a language, with everything that language needs declared together.
- Behavior for Rust and TypeScript/JavaScript identical after the change, proven by the existing suites rather than by new tests written to match new behavior.
- Storage that does not need a schema change when the registry grows.
- A frontend that renders language controls from data, so `add-lsp-go-python-cpp` and `add-lsp-java-jdtls` add no per-language component.

**Non-Goals:**

- User-defined languages. The registry is a build-time table, not user data (see Decision 1).
- Any new language. Adding one here would hide a registry regression behind new-feature noise.
- Per-workspace argument templating. `jdtls` needs it; it is designed in `add-lsp-java-jdtls` once there is a real second shape to generalize against.

## Decisions

### 1. The registry is a static table in the domain, not a database table

A language entry declares its id, candidate executable names in preference order, project-root markers, extension-to-`languageId` mapping, default startup arguments, default initialization options, platform applicability, and its server-test fixture project.

Rejected: storing the language set in SQLite so users could add languages. Every registered language needs a fixture project, root-detection rules, and extension mapping that only code can supply. A user-declared language would be a row the runtime cannot actually serve, and the failure would surface as a confusing server-start error rather than as "unsupported". Generic user-supplied LSP servers are a different feature with a different threat model.

The database keeps storing *configuration for* languages. It stops storing *which languages exist*.

### 2. Language id becomes a validated string newtype

Following `CliToolId`: constructed through one validated type, rejected at every boundary (wire DTO, SQLite row) if malformed.

Rejected: keeping an enum and widening it. That is the status quo whose cost this change exists to remove — every new variant reopens 36 match sites.

Trade-off: the compiler stops proving match exhaustiveness over languages. Mitigation: registry lookup returns `Option`, an unregistered id is a safe rejection rather than a panic or a silent default, and a registry-completeness test asserts every entry supplies markers, extensions, and a fixture project.

### 3. The `CHECK` constraint is removed by rebuilding the table, preserving revisions

One migration: create the replacement table without the constraint, copy every row, drop the original, rename.

The `revision` columns must be copied verbatim, not reset. Configuration revision feeds the server-instance configuration fingerprint, and `lsp-server-management` requires that a changed fingerprint drains and restarts matching servers. Resetting revisions on upgrade would make every running server look stale and restart on next launch — a visible regression produced by a migration that "worked".

### 4. Unknown language ids in storage are preserved, not deleted

Once the constraint is gone, a row can name a language the running build does not register — after a downgrade, or if a later build retires a language. The effective configuration excludes such rows; the rows themselves stay untouched.

Rejected: deleting unknown rows during migration or load. A downgrade-then-upgrade cycle would silently discard the user's settings for the languages that were temporarily unknown.

### 5. Startup arguments become real configuration, with unset distinct from empty

The registry declares defaults. A user override *replaces* the defaults rather than appending, because appending makes a default such as `--stdio` impossible to remove.

That requires distinguishing "not overridden" from "overridden to nothing". The column is currently `NOT NULL DEFAULT '[]'`, which conflates them, so the rebuilt table makes it nullable: `NULL` means use the registry default, a JSON array means use exactly that. Without this distinction, clearing the field in the settings form would strip `--stdio` from the TypeScript server and break it in a way that looks like a discovery bug.

Startup arguments must also enter the configuration fingerprint, or changing them would not restart the affected servers.

### 6. The frontend contract carries language descriptors, not a widened enum

The backend supplies a list of descriptors (id, platform applicability, whether an executable override is permitted) alongside configuration. `LspLanguageId` becomes an opaque string at the contract boundary.

Rejected: generating a TypeScript union from the Rust registry. The repository has no codegen step for this contract, and a generated union would re-freeze the set at build time — the same problem in a new place.

Display names keep the existing `lspSettings.language.<id>` i18n key convention, with fallback to the raw id when a locale lacks the key. Without the fallback, adding a language in a later change breaks five locale bundles simultaneously and renders blank labels until all are updated.

### 7. Settings render one card per descriptor

A single language-card component is rendered per descriptor, replacing the two hard-coded language sections. This is also why the frontend diff should be net-neutral or negative in line count, which matters because the services subtree has an aggregate budget enforced only by `npm run architecture:check`.

## Risks / Trade-offs

- **Table rebuild resets revisions and mass-restarts servers on first launch** → copy `revision` and `updated_at` verbatim; add a migration fixture test that asserts a pre-migration row's revision survives.
- **Migration number collides with another branch** → every worktree shares one `%APPDATA%\ai.vanehub.app\vanehub.sqlite`, so a colliding number produces a startup crash that reads like a missing table. Scan sibling branches for in-flight numbers before choosing, and expect four hard-coded version assertions to need updating — neither the compiler nor clippy reports them.
- **Loss of exhaustive matching hides a half-migrated code path** → the registry-completeness test plus `Option`-returning lookups; a language that reaches runtime without markers, extensions, or a fixture fails the test rather than failing a user.
- **A subtle behavior change slips in under "pure generalization"** → acceptance is the existing native LSP end-to-end suite, the `code_intelligence` unit tests, the LSP Playwright spec, and the `domain-lsp` desktop layer all passing unchanged. New tests are added only for genuinely new behavior: unknown-id handling, startup-argument validation, and preference-order selection.
- **Frontend type widening ripples further than expected** → `LspLanguageId` is referenced from the service boundary, both adapters, the contract module, and the settings pages. Do the widening in one commit so `tsc --noEmit` enumerates the sites rather than discovering them one at a time.
- **`architecture:check` fails last, after lint, tsc, and build have passed** → run it before declaring the change done, not as part of final verification.

## Migration Plan

1. Choose the migration number after scanning sibling branches; update the four hard-coded version assertions in the same commit.
2. Rebuild `lsp_language_configurations` without the `CHECK` constraint and with a nullable `startup_arguments_json`, copying all columns including `revision` and `updated_at`.
3. Land the Rust registry and the id newtype, keeping the two existing languages' declared data byte-identical to today's constants.
4. Widen the service contract and both adapters together, then convert the settings section to data-driven cards.

Rollback: the change is one migration plus code. Rolling back the code without rolling back the migration leaves a table with no `CHECK` and a nullable column, which the previous build reads correctly — its own `INSERT OR IGNORE` seeding and reads do not depend on the constraint. A row for an unregistered language written after the upgrade would be ignored by the older build for the same reason Decision 4 gives.

## Open Questions

- Whether the bounded per-request deadline should become a per-language registry field rather than a global constant. `jdtls` is much slower to initialize than the current servers and may force this, but nothing in this change depends on the answer, and deferring it changes neither these specs nor this task breakdown.
