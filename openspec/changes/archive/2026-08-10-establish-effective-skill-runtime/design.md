## Context

See `proposal.md` for motivation and the delta specifications for observable behavior.

The current Skill context already separates domain, application, infrastructure, and Tauri API concerns under `src-tauri/src/contexts/tooling/skills/`. It stores Global and Workspace records in SQLite, materializes six built-ins into the user's Global Skill directory, manages CLI mount bindings and API prompt bindings, and assembles eagerly injected Skill text for native API agents. The frontend exposes Skill operations through `src/services/agent-service.ts`, with Tauri and Web/mock adapters, and the Settings UI consumes those contracts.

This change crosses package discovery, domain identity, persistence, prompt assembly, native tool dispatch, migration, and UI presentation. It must preserve existing bindings, tombstones, enabled state, user edits, prompt budgets, Plan mode safety, and unified logging requirements.

## Goals / Non-Goals

**Goals:**

- Introduce an effective catalog without conflating management scope, storage layer, provenance, type, delivery, trust, or availability.
- Preserve eager behavior for existing valid Skills while enabling explicit on-demand Role Skills.
- Make shipped Skills immutable and migrate legacy mutable built-ins without data loss.
- Add bounded read-only discovery, loading, and resource access to the existing native agent tool loop.
- Keep desktop and Web/mock service contracts behaviorally aligned.
- Establish stable extension points for later overlays, delegated Utility execution, configuration, registry installation, and evolution workflows.

**Non-Goals:**

- Executing Utility Skills as delegated sub-agents.
- Executing or dynamically registering scripts bundled in a Skill.
- Downloading, installing, or updating packages from a remote registry.
- Editing System packages through overlays or applying learned guidance.
- Adding the expanded built-in catalog, Curator, candidate generation, or automatic evolution.
- Changing CLI Agent-owned Skill discovery semantics beyond resolving the effective source used by existing bindings.

## Decisions

### 1. Model scope, layer, origin, classification, trust, and availability separately

The Rust domain adds independent value types:

```text
SkillType        = Role | Utility
SkillDelivery    = Eager | OnDemand
SkillLayer       = Project | User | Registry | System
SkillOrigin      = Created | Imported | Installed | Shipped | Migrated
SkillTrust       = Trusted | Untrusted
SkillAvailability = Available | Disabled | Invalid | Conflicting | Unsupported
```

`SkillScope` remains the caller-facing management/binding context (`global` or `workspace`) for compatibility. It does not decide precedence by itself. A `SkillDefinition` represents one package in one layer; an `EffectiveSkill` contains the selected definition plus shadowed definitions and compatibility annotations.

Metadata parsing accepts optional `type`, `delivery`, and `aliases`. Missing type and delivery use a recorded compatibility default of `Role + Eager`; newly created Skills should write explicit values. Unknown enum values make the definition unavailable instead of silently changing behavior. Utility definitions remain visible but `Unsupported` until delegated execution is added.

Alternatives considered:

- Reuse `SkillScope` for all four layers. Rejected because management scope and resolution precedence have different lifecycles and would make existing binding APIs ambiguous.
- Derive delivery from type. Rejected because Role Skills can validly be eager or on-demand, while Utility execution is a separate capability gate.
- Reject all legacy metadata. Rejected because it would silently remove current prompt behavior after upgrade.

### 2. Build an effective catalog from layer providers, then apply bindings

The application layer receives a `SkillLayerProvider` port for each source. Providers enumerate validated package descriptors rather than exposing arbitrary filesystem access. Resolution proceeds as follows:

1. Canonicalize the active workspace, if any.
2. Enumerate Project definitions within that workspace to a maximum package discovery depth of three.
3. Enumerate User definitions under the fixed home Skill root.
4. Enumerate locally installed Registry definitions from application-managed package storage; this provider is empty until installation is implemented.
5. Enumerate compiled or application-resource System packages.
6. Group definitions by canonical id and select the first valid precedence winner: Project, User, Registry, System.
7. Record lower definitions as shadowed and same-layer collisions as deterministic conflicts.
8. Resolve aliases after canonical winners are known; exact canonical ids win and ambiguous aliases fail closed.
9. Apply deletion intent, enabled state, and Agent bindings from existing persistence to form the session-specific effective view.

Discovery skips known high-volume or unsafe directories and does not follow links outside a layer root. Results are ordered by layer rank and canonical id. The catalog cache key includes canonical workspace, package inventory fingerprint, and persisted state revision. Mutations invalidate the relevant key; startup and drift reconciliation can rebuild it.

The Registry layer is deliberately modeled now even though its provider returns no network-installed content in this change. This avoids another domain migration when a later installation service is added.

Alternatives considered:

- Copy every package into one merged directory. Rejected because it destroys provenance, makes shadowing unauditable, and risks overwriting user content.
- Resolve only at UI listing time. Rejected because prompt assembly and tools require the same winner and must not implement separate precedence rules.
- Load every discovered definition and let the model choose. Rejected because it is non-deterministic and leaks shadowed instructions into context.

### 3. Package shipped Skills as immutable System resources

The six shipped packages move to a versioned resource tree owned by the application build. A manifest records canonical id, package version, hashes, aliases, classification, and resource entries. The System provider reads from this manifest and exposes content through the same package-reader port as filesystem providers.

System package paths are never returned to models or treated as mutable source directories. Existing edit, delete, import, and direct restore paths check layer mutability in the application service before beginning a filesystem transaction. Enablement, binding, deletion intent, and usage remain mutable state outside the package.

The implementation should reuse the existing build/resource mechanism and avoid a new runtime scripting or archive dependency. Tests use an in-memory System provider so package behavior can be validated without depending on installation paths.

Alternatives considered:

- Continue materializing built-ins under `~/.vanehub/skills`. Rejected because shipped content remains indistinguishable from user content and cannot be upgraded safely.
- Store shipped Markdown only in SQLite. Rejected because multi-file package resources would be awkward, content review would degrade, and package hashes would be less transparent.
- Allow direct System edits and restore from a backup. Rejected because updates and repair would again risk overwriting local changes.

### 4. Treat bindings as references to canonical identity, not a physical layer

Existing bindings and enabled state continue to address the stable Skill id and management context. At use time the effective catalog resolves that reference to the current winning definition. A higher-priority definition therefore changes content without requiring every Agent binding to be rewritten.

Responses add effective layer, origin, type, delivery, availability, aliases, compatibility state, and bounded shadow summaries. Existing fields remain during a compatibility window so current frontend code and stored records can be migrated additively.

Project bindings remain keyed by canonical workspace. Without a workspace, Project definitions and Workspace bindings do not participate. CLI mount reconciliation uses the effective filesystem-backed package when one exists; a System package requiring a physical CLI mount is materialized only into an application-managed read-only cache, never into the User layer. Cache files are derived artifacts and are not catalog definitions.

Alternatives considered:

- Bind to `(layer, id)`. Rejected as the default because an override would require rebinding and restoration would be surprising. Layer-specific inspection remains available in management responses.
- Recreate bindings during every migration. Rejected because it increases transactional risk and changes user intent.

### 5. Keep prompt assembly compatibility-first and delivery-aware

The current prompt loader is changed to request effective Skills for the active Agent and workspace. It filters in this order:

1. canonical binding and workspace applicability;
2. effective winner only;
3. enabled and available;
4. `type == Role`;
5. `delivery == Eager`;
6. existing 8,000-character per-Skill and 16,000-character aggregate budgets.

Ordering remains deterministic using layer rank, canonical workspace, and canonical id. Legacy metadata maps to eager Role behavior. Explicit on-demand Role Skills and all Utility Skills are excluded. Successful final inclusion bumps use activity once per Skill and generation; skipped Skills do not.

Usage writes occur after prompt selection and are best-effort. A tracking failure is logged safely and does not fail generation.

Alternatives considered:

- Switch every Skill to on-demand immediately. Rejected because existing sessions would lose established instructions and behavior.
- Inject a short summary for Utility Skills. Rejected because it blurs the Role/Utility security boundary before delegation exists.
- Truncate eager bodies. Rejected to preserve the current whole-instruction budget contract.

### 6. Add three stable tools instead of one tool per Skill

The native provider-agnostic catalog adds fixed definitions:

- `list_skills`: bounded filters and metadata only.
- `load_skill`: canonical id or alias; returns effective Role instructions and resource index.
- `read_skill_resource`: reads a logical URI previously derivable from effective package metadata.

The tools are translated through the existing interface-format adapters and dispatched in the existing multi-turn tool loop. Their schemas do not depend on installed Skills, preserving provider prompt/tool cache stability and preventing unbounded tool catalogs.

All three are read-only and remain available in Plan mode. Authorization is enforced both when building the offered catalog and again during dispatch. `load_skill` refuses Utility Skills until delegation exists; it does not reinterpret them as Role Skills.

The Web/mock adapter implements representative list/load/read outcomes through the same frontend/runtime contract used by browser chat simulation. It must model immutable, shadowed, truncated, and unavailable results rather than returning static happy-path data only.

Alternatives considered:

- Dynamically register every Skill as a provider tool. Rejected because schemas and tool counts would change with inventory, weaken cache reuse, and mix instructions with executable authority.
- Reuse the generic file tool. Rejected because it exposes host paths and cannot enforce effective-package identity or indexed-resource boundaries.
- Inject all resource contents from `load_skill`. Rejected because it defeats progressive disclosure and creates unpredictable token use.

### 7. Use logical URIs and a package reader for progressive disclosure

Models see logical identifiers of the form:

```text
skill://<canonical-id>/
skill://<canonical-id>/references/example.md
```

The returned URI identifies the current effective package in the active session context; it is not a stable host path. Each read resolves the canonical id again and validates that the requested relative path exists in the bounded resource index. If the effective winner changes, stale resource requests fail with a refresh-required result rather than reading the old layer implicitly.

`load_skill` returns no more than 12,000 Unicode characters, a `truncated` flag, content hash/revision, logical base URI, and a deterministic index for `scripts/`, `references/`, `templates/`, and `assets/`. `{skill_base_dir}` is replaced by the logical base URI. Index counts, path lengths, entry counts, and read output are bounded by shared constants.

The reader canonicalizes host paths internally, rejects absolute and parent paths, hidden components, package escapes, escaping links, binary content, and oversized files. Files under `scripts/` are data only in this change. Diagnostics log identity, operation, size, and reason codes, never bodies or unrestricted paths.

Alternatives considered:

- Return absolute paths. Rejected because paths leak environment details and could be reused outside the package sandbox.
- Bind URIs permanently to a layer. Rejected because agent operations should use the effective definition; management inspection can use separate layer-aware APIs.
- Permit arbitrary package-relative reads without an index. Rejected because bounded enumeration provides a smaller, testable authority surface.

### 8. Store usage in recoverable sidecars separate from package content

Usage records are keyed by canonical id and effective layer identity and include view/use counters plus last-viewed/last-used timestamps. The Project sidecar lives at `{workspace}/.vanehub/skills/.usage.json`; non-project activity lives at `~/.vanehub/skills/.usage.json`. System and Registry packages therefore remain immutable.

Writes use the existing filesystem transaction primitives: read current revision, update in memory, write a temporary file, and atomically replace. An in-process lock serializes writers; the stored revision supports compare-and-swap detection for later cross-process use. Corrupt files are moved to a timestamped bounded backup and replaced with an empty valid document. Cleanup limits retained backup count and total size.

`bump_view` occurs only after a successful `load_skill`; `bump_use` occurs only after final eager prompt inclusion. Patch and overlay counters are reserved in the versioned sidecar schema but are not mutated by this change.

Alternatives considered:

- Put counters in `SKILL.md`. Rejected because loading would mutate package content and System packages must remain immutable.
- Put all counters in SQLite. Rejected for this phase because project-portable usage belongs with project state and the requested sidecar boundary also supports later tooling. SQLite may index aggregate projections later without becoming authoritative.
- Fail Skill loading when telemetry fails. Rejected because usage is secondary and must not reduce agent availability.

### 9. Perform an idempotent, content-aware migration

Migration is a versioned startup reconciliation executed before effective catalog use:

1. Load the System manifest and existing built-in tombstones, records, bindings, and enabled state.
2. Inspect each legacy built-in source without modifying it.
3. If the source hash equals the shipped package hash, record System as authoritative and stage the redundant source for recoverable cleanup.
4. If valid content differs, preserve the directory as a User-layer override with `Migrated` origin.
5. If content is invalid or unreadable, leave it untouched, mark the definition unavailable, and report a per-Skill failure.
6. Preserve deletion intent and disabled state separately so System content is not resurrected as effective.
7. Commit database state and filesystem changes using the existing transaction/recovery model.
8. Emit one redacted reconciliation summary and bounded per-Skill outcomes through unified logging.

The migration stores a version and per-Skill result so retries are safe. A crash before commit leaves legacy files authoritative under the old schema; a crash during recoverable cleanup is completed or rolled back by existing transaction recovery. No valid divergent file is deleted automatically.

Restore now clears the legacy deletion intent and reveals the System package or a higher-layer winner. It does not recreate a mutable shipped directory.

Alternatives considered:

- Overwrite all legacy built-ins with the System version. Rejected because it destroys user edits.
- Treat every legacy file as an override. Rejected because unchanged copies would permanently shadow future System updates.
- Delete identical files before committing state. Rejected because a crash could temporarily remove the only known definition.

### 10. Extend service contracts additively and keep UI runtime-neutral

Rust Tauri commands expose effective inventory and preview data through the Skill API. `src/services/agent-service.ts` defines the shared TypeScript types and service methods; `tauri-agent-client.ts` is the only frontend layer that invokes native commands; `web-agent-client.ts` provides equivalent mock behavior. React hooks and components consume only the service interface.

The Settings inventory remains one effective row per canonical id. Details show shadowed definitions, compatibility defaults, and unavailable reasons. System rows allow preview, enablement, and binding actions but suppress direct edit/delete. User and Project definitions retain conflict-aware mutation where the current management context permits it.

UI labels distinguish management scope from effective layer. Existing Global/Workspace tabs can remain initially, but their copy and summaries must not imply that System packages are mutable Global files.

Alternatives considered:

- Add Tauri calls directly to new dialogs. Rejected by the project service-boundary rule and because it would break Web mode.
- Render every layer as a separate active card. Rejected because users need to understand the effective behavior first; shadowed definitions belong in details.

### 11. Test at domain, adapter, migration, security, and UI boundaries

Rust domain tests cover precedence, same-layer collisions, exact-id versus alias behavior, compatibility defaults, and availability. Application tests cover binding resolution, prompt filters, usage events, migration outcomes, and idempotency. Filesystem tests cover traversal, links, binary/size limits, corrupt sidecar recovery, and atomic writes. Native tool-loop tests cover both provider formats, malformed inputs, Plan mode, round limits, and visible persisted results.

Frontend contract tests keep both adapters aligned. Settings component tests cover immutable controls, effective/shadowed labels, unavailable Utility state, migration diagnostics, and accessibility. Existing prompt-injection, drift, recovery, and binding suites remain regression gates.

## Risks / Trade-offs

- [Legacy content is misclassified as unchanged and removed] → Compare normalized package manifests and hashes, retain recoverable migration backups, and test edited frontmatter, body, and resource cases independently.
- [Four-layer discovery slows every generation] → Cache effective catalogs by workspace and state revision, invalidate on managed mutations, and keep bounded discovery outside the provider request hot path.
- [Aliases create surprising winner changes] → Resolve canonical ids first, reject ambiguous aliases, expose canonical identity in every result, and log safe conflict diagnostics.
- [Logical URIs become stale after an override changes] → Include content revision in load results and return refresh-required errors when the effective package no longer matches.
- [Usage sidecar contention loses counts] → Serialize in-process writes, use revision checks and atomic replacement, and keep telemetry best-effort.
- [System immutability removes a familiar edit workflow before overlays exist] → Preserve explicit User-layer creation/import and clearly explain the temporary customization path in the UI.
- [Registry appears implemented when only the layer exists] → Report the provider as locally empty and do not expose install/update actions in this change.
- [Utility metadata suggests executable capability] → Mark Utility Skills unsupported, exclude them from prompt injection, and make `load_skill` refuse them until delegation is delivered.
- [Resource indexes expose sensitive filenames] → Restrict directories and entry counts, reject hidden components, return logical paths only, and never log resource contents.

## Migration Plan

1. Add the domain enums, compatibility parsing, effective models, and provider ports behind tests while leaving current prompt assembly active.
2. Add the System package manifest and package reader for the existing six built-ins.
3. Add persistence migrations for reconciliation version, effective metadata, and preserved deletion/enablement state.
4. Run content-aware legacy reconciliation and verify per-Skill outcomes before switching reads to the effective catalog.
5. Switch list, preview, binding resolution, drift, and eager prompt assembly to the effective catalog while retaining legacy response fields.
6. Add usage sidecars and fixed Skill tools, then enable them in native normal and Plan mode catalogs.
7. Update shared frontend contracts, both adapters, Settings presentation, localization, and tests.
8. Run strict OpenSpec validation and the repository's full frontend, Rust, contract, coverage, and UI verification suite appropriate to the implementation.

Rollback keeps the migration version and recoverable legacy-file backups. Before any user creates a new higher-layer override under the new runtime, a rollback can restore unchanged legacy built-in files and continue using prior records. After new-runtime mutations exist, rollback must leave those directories untouched and disable the new catalog path through a versioned compatibility switch; it must never overwrite or delete user content. Database migrations remain additive so older binaries can ignore new columns and tables where supported.

