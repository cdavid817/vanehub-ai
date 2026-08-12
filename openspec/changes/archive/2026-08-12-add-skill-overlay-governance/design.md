## Context

See `proposal.md` for motivation and the delta specifications for behavior. This change assumes `establish-effective-skill-runtime` is implemented first: Overlay replay needs canonical Skill identity, effective package resolution, logical resource URIs, immutable System packages, pinned state, filesystem transactions, and usage sidecars.

The Overlay subsystem becomes the only supported mutation boundary for future learned guidance and automated Skill evolution. It therefore needs stricter trust, concurrency, history, recovery, and path rules than ordinary mutable User or Project Skill editing. The same replay result must be consumed by eager prompt assembly, on-demand loading, CLI-derived mounts, previews, and resource reads.

## Goals / Non-Goals

**Goals:**

- Preserve authoritative packages while producing one deterministic Overlay-applied effective view.
- Support exact instruction patches, appended guidance, and bounded supporting-resource overrides.
- Make every mutation reversible, conflict-aware, attributable, and recoverable.
- Quarantine imported content until explicit trust promotion.
- Fail safely when base content drifts or replay becomes ambiguous.
- Expose governance through shared desktop and Web/mock service contracts.
- Provide a stable write target for later Curator and automatic-evolution changes.

**Non-Goals:**

- Generating candidates, assessing candidate quality, or applying mutations without a direct user action.
- Executing Overlay files or registering them as tools.
- Modifying base `SKILL.md` files or immutable System package resources.
- Synchronizing Overlays through a remote account or registry.
- Providing arbitrary three-way text merging outside the constrained reconciliation workflow.
- Allowing trust promotion to bypass deterministic security checks.

## Decisions

### 1. Use a manifest plus payload store, not modified package copies

Each `(canonical_skill_id, overlay_scope, canonical_workspace?)` has one versioned `OverlayDocument` manifest. Large supporting payloads are stored beside the manifest and referenced by content hash; instruction mutations remain in the JSON document for reviewable diffs.

Core domain shapes are:

```text
OverlayDocument
├─ schema_version
├─ canonical_skill_id
├─ scope: System | User | Project
├─ workspace_identity?
├─ revision
├─ base_identity
├─ base_instruction_hash
├─ base_package_hash
├─ trust
├─ patches[]
├─ learn_blocks[]
├─ files[]
├─ conflicts[]
├─ created_at / updated_at
└─ prior_revision_hash

OverlayPatch
├─ id, old_string, new_string, replace_all
├─ state: Active | Disabled | Reverted
├─ creation_base_hash
└─ created_at / updated_at

OverlayLearnBlock
├─ id, guidance, state
└─ created_at / updated_at

OverlayFile
├─ id, logical_path, media_type, size, content_hash
├─ payload_ref, state
└─ created_at / updated_at

OverlayConflict
├─ id, mutation_id, reason, witnessed_base_hash
├─ state: Active | Resolved | Ignored
└─ resolution_revision?

OverlayTrust
├─ state: Trusted | Untrusted
├─ origin: Local | Imported
├─ source_summary?
├─ reviewed_revision?
└─ reviewed_content_hash?
```

Payloads are content-addressed within the Overlay boundary, never referenced by caller-supplied absolute paths. The manifest is the transaction root: a payload is not active until a committed manifest revision references it. Unreferenced payload cleanup occurs only after recovery has established that no committed or backup revision needs it.

Alternatives considered:

- Copy and edit the complete Skill package. Rejected because it hides the base/override distinction and makes upgrades and rollback destructive.
- Store all binary assets as base64 inside JSON. Rejected because it inflates manifests and makes CAS, diff, and size enforcement unnecessarily expensive.
- Store mutations only in SQLite. Rejected because project Overlays must remain project-scoped and auditable alongside project state; SQLite can index summaries but is not the source of truth.

### 2. Keep Overlay scope distinct from Skill layer

Overlay scope expresses where customization applies, not where the base package came from:

- System Overlay: global VaneHub customization applied first.
- User Overlay: user-specific customization applied second.
- Project Overlay: canonical-workspace customization applied last.

A System Overlay may target a User-, Registry-, Project-, or System-layer package. It does not mutate or change the layer of that package. Project Overlay lookup always requires the canonical workspace identity used by the effective catalog.

Storage layout:

```text
~/.vanehub/skill_overlays/<skill-id>.json
~/.vanehub/skill_overlays/user/<skill-id>.json
~/.vanehub/skill_overlays/.payloads/...
~/.vanehub/skill_overlays/history/<skill-id>/events-<sequence>.jsonl

<workspace>/.vanehub/skills/.overlays/<skill-id>.json
<workspace>/.vanehub/skills/.overlays/.payloads/...
<workspace>/.vanehub/skills/.overlays/history/<skill-id>/events-<sequence>.jsonl
```

System and User history events include their scope in each event because they share the home Overlay root. Internal reserved directories are excluded from manifest discovery.

Alternatives considered:

- Map Overlay scopes directly onto Project/User/Registry/System Skill layers. Rejected because users must be able to customize a shipped package for one project without copying or relabeling it.
- Use only User and Project scopes. Rejected because global governance and future administrator-curated guidance need a lower-precedence customization tier distinct from personal learning.

### 3. Replay a deterministic scope chain and fail one scope at a time

The replay engine accepts an immutable `EffectiveSkillPackageSnapshot` and applicable Overlay snapshots. It never reads live files during string replacement. Processing is:

1. Verify base identity, package hash, trust, pinned snapshot, document revision, and payload hashes.
2. Start with base instructions and resources.
3. For System, User, then Project Overlay:
   - replay active patches in creation order;
   - append active guidance blocks to the single delimited guidance section;
   - apply active resource entries by logical path;
   - produce a scope result and hashes.
4. If a scope has an unresolved conflict or integrity failure, discard that scope's complete tentative result and continue from the last healthy lower-scope result.
5. Do not apply higher scopes whose base chain depends on the failed scope; mark them blocked by an earlier scope to avoid surprising patches against different text.
6. Return base, last healthy effective snapshot, per-scope status, diff summary, and conflict information.

Exact patches use Unicode string equality without whitespace or newline normalization. `replace_all = false` requires exactly one match; `replace_all = true` requires at least one. This makes preview and commit results identical for the same witnesses.

Guidance blocks are rendered under one stable delimiter. Their stored text does not include the delimiter, preventing a block from escaping or forging the section structure. Resource resolution uses the same scope order and records shadowed sources.

Alternatives considered:

- Best-effort application of patches within a conflicted scope. Rejected because users could receive a partially evolved instruction set that was never reviewed as a whole.
- Continue applying higher scopes after a lower failure. Rejected because their exact-match assumptions may depend on lower-scope text.
- Fuzzy patch matching. Rejected because it can silently modify the wrong instruction and makes deterministic reconciliation impossible.

### 4. Separate preview from commit but run the same validation pipeline

Preview and mutation use one pure preparation pipeline:

```text
request
→ canonical identity and scope validation
→ pinned check
→ size/path/media validation
→ secret and injection scan
→ expected revision and base witness check
→ tentative document revision
→ deterministic replay
→ effective diff and conflict result
```

Preview stops before persistence and returns witnesses required for commit. Commit reruns every check against live snapshots; it does not trust the preview result. This closes the time-of-check/time-of-use gap while allowing the UI to retain user input after a stale response.

Each mutation request carries `expected_overlay_revision`, `expected_base_package_hash`, and, for payload changes, `expected_payload_hash` where relevant. Import promotion also carries the exact imported revision and whole-document hash that the user reviewed.

Alternatives considered:

- Commit a preview token without revalidation. Rejected because base files, pin state, or Overlay revisions may change between calls.
- Automatically rebase stale edits. Rejected because it could approve a diff the user did not see.

### 5. Treat local creation and imported content differently

Local mutations become trusted only after all validation succeeds and the transaction commits. Imports are parsed in a staging area, bounded by compressed size, expanded size, entry count, path depth, per-file size, and total mutation count. Hard-deny content is rejected before durable import state. A valid import is stored untrusted and excluded from replay.

The first import profile is ZIP v1 with a deliberately closed layout:

```text
overlay.json
payloads/sha256/<lowercase-sha256>
```

`overlay.json` is the only manifest entry. Payload entry names derive from the manifest's `sha256/<lowercase-sha256>` references rather than caller paths. The importer requires an exact one-to-one closure between active or retained manifest file records and archive payload entries: missing, duplicate, hash-mismatched, size-mismatched, or unreferenced entries reject the whole package. Directory placeholders are permitted only when empty and on the path to declared entries; links, encrypted entries, unsupported compression methods, extra roots, and trailing data are rejected.

Parsing occurs in a new unpredictable quarantine directory beneath the Overlay transaction boundary. Entry metadata is validated before extraction, writes use newly created files without following links, and the complete extracted set is rescanned from quarantine before any manifest, payload, history, or usage transaction is prepared. Cleanup runs after both success and failure. Incoming trust fields are non-authoritative: after structural and security validation, the imported document is normalized to origin `Imported`, state `Untrusted`, and empty review witnesses before its document hash and durable revision are computed.

Promotion requires explicit UI review plus exact revision and document-hash witnesses. Promotion reruns scanning, payload verification, base comparison, and replay. Any change invalidates the review. Trust is attached to a revision, not permanently to a source.

No UI or API provides a “trust anyway” bypass for hard-deny rules. False positives are handled by editing the content into a safe representation and resubmitting, leaving the rejected body only in unsaved caller state.

Alternatives considered:

- Trust imports from a known path or registry automatically. Rejected because filesystem location is not a content trust proof.
- Store rejected secret-bearing imports for later review. Rejected because quarantine would itself persist dangerous content.
- Mark the Overlay trusted forever after one approval. Rejected because subsequent imported changes would inherit authority without review.
- Accept multiple archive formats. Rejected because format sniffing expands the parser attack surface and makes deterministic export and Web/mock parity harder to preserve.
- Allow arbitrary supporting-file paths in the ZIP. Rejected because content-addressed archive names avoid path authority and let the manifest remain the sole logical-resource mapping.

### 6. Use deterministic scanners before content reaches durable state

The scanner returns rule ids and byte/character ranges internally but external results contain only safe rule ids, counts, and remediation text. Initial hard-deny families include:

- private-key and credential structures;
- prompt-authority override phrases;
- script tags and executable markup;
- executable signatures and disallowed extensions;
- traversal, hidden, absolute, alternate-stream, and reserved-device paths.

Text is decoded with strict UTF-8 for instructions, patches, guidance, references, and templates. Assets may be approved binary media after extension, declared media type, magic signature, and size agree; they are never returned by text-only resource reads. Detection happens before unified logging, so raw rejected content cannot enter logs.

Scanner rules are versioned in history events. A future scanner rule change can mark an existing trusted Overlay `needs_rescan`, but this change does not silently disable previously active content unless an executable signature or integrity failure is detected.

Alternatives considered:

- Rely on model review for injection detection. Rejected because the mutation boundary needs deterministic, offline enforcement.
- Block every binary file. Rejected because legitimate image assets are part of supported Skill packages.

### 7. Make base drift an explicit reconciliation state

The document records both instruction and complete package hashes. When either differs from the current effective base, the service computes a reconciliation preview without updating witnesses:

- clean replay: all mutations still apply, but confirmation is required to accept the new base witness;
- conflict: exact patch or resource expectation fails;
- blocked: an earlier Overlay scope is unhealthy;
- integrity failure: document or payload hashes do not verify.

Reconciliation is a normal CAS mutation producing a new revision. Users may edit a mutation, disable it by ignoring the conflict, or cancel. Resolved and ignored conflicts remain in the document and append-only history. No reconciliation operation changes the base package.

Agent consumption uses the last deterministic lower-scope or base result while reconciliation is unresolved. It never uses a newly drifted clean replay until the user confirms the new base witness, because that diff was not previously approved.

Alternatives considered:

- Automatically accept clean replay after a base update. Rejected because exact text still may have changed meaning even when matches remain.
- Disable every Overlay globally when one scope drifts. Rejected because healthy lower-scope customization can remain deterministic and useful.

### 8. Pinning freezes the current Overlay revision

Pinning does not remove already active customization. It freezes the effective Overlay chain at its committed revisions and rejects all Overlay mutations, including import, promotion, trust changes, disable, revert, and reconciliation. The user must explicitly unpin before changing effective content.

This definition avoids the surprising behavior where pinning immediately removes active guidance. It also keeps `pinned_refusal` simple and auditable. If base drift occurs while pinned, the last safe replay policy still applies; resolution requires unpinning.

Alternatives considered:

- Allow disable and revert while pinned. Rejected because those operations change effective instructions and weaken the meaning of pinning.
- Stop replaying existing Overlays on pin. Rejected because pinning would itself become a destructive content change.

### 9. Commit manifest, payload, history, and usage as one recoverable operation

The existing Skill filesystem transaction mechanism is extended with an Overlay transaction plan. Under a per-Overlay in-process lock, commit stages payload additions, the next manifest, one append-only history event, and the updated usage sidecar. It verifies witnesses again, writes temporary files, fsyncs where supported, and swaps a transaction marker through committed and cleanup states.

If usage-sidecar persistence fails, the Overlay mutation does not commit: patch and Overlay mutation counts are governance audit data rather than optional telemetry for this path. Recovery uses the marker to finish the whole next revision or restore the prior manifest, history length, payload reachability, and usage revision.

History events form a hash chain. Active JSONL segments stop before 4 MiB, close with a segment hash, and link the next segment to it. Normal operations never rewrite or delete closed segments. History read verifies event and segment linkage and reports corruption; it does not silently reconstruct missing events from the current manifest.

Alternatives considered:

- Make counters best-effort as ordinary views are. Rejected because successful governance mutations must agree with their audit counters.
- Append history after manifest commit. Rejected because a crash could create an unaudited effective change.
- Keep one unlimited JSONL file. Rejected because reads, recovery, and corruption scope would grow without bound.

### 10. Add an Overlay application boundary, not UI filesystem access

The Rust Skill application service gains operations grouped around query, preview, mutation, trust, history, and reconciliation. Ports abstract the Overlay repository, package snapshot reader, scanner, clock, unified logger, pin/usage state, and transaction executor. Tauri commands convert boundary failures to structured serializable errors.

The shared frontend contract includes:

- bounded Overlay summary and detail;
- base/effective diff and resource shadowing;
- preview and commit requests with witnesses;
- trust/import review results;
- conflict and reconciliation models;
- paginated verified history;
- typed validation, stale, pinned, limit, trust, and integrity errors.

`tauri-agent-client.ts` contains all native invocation mappings. `web-agent-client.ts` simulates revision advancement, stale witnesses, untrusted import, conflict, pinning, and revert with the same response shapes. React hooks and components do not read Overlay files or call Tauri directly.

Alternatives considered:

- Expose generic Overlay filesystem commands. Rejected because callers could bypass scanning, witnesses, and transaction invariants.
- Let Web mode omit mutation behavior. Rejected because the settings UI must remain testable and behaviorally coherent through its service adapter.

### 11. Integrate the Overlay workspace into existing Skill details

The Settings UI keeps one effective Skill row and adds an Overlay area within details. It does not create a second “evolved Skill” inventory. The area has:

- status header: active scopes, trust, pinned, revisions, conflicts;
- base/effective diff;
- patches and learned-guidance lists;
- resource override and shadowing list;
- import/trust review;
- reconciliation workspace;
- verified history timeline and revert actions.

Patch and guidance forms always preview before commit. Reconciliation uses witnessed base, current base, and proposed effective output, with per-conflict edit or ignore choices followed by a complete final preview. Stale errors retain unsaved form state. System base packages show the same Overlay actions because Overlay mutability is independent of base immutability.

Large diffs and histories are paginated or virtualized and bounded at the service layer. UI copy distinguishes “base package,” “Overlay,” and “effective content.” All dialogs use localized accessible names, focus containment, keyboard-safe dismissal, and focus restoration.

Alternatives considered:

- Put all governance only in a global Curator page. Rejected because users need base and effective context for one Skill even before Curator exists.
- Show raw Overlay JSON as the primary editor. Rejected because it invites invalid revisions and makes safe diff review difficult.

### 12. Test replay and recovery as security-critical code

Property-oriented tests generate patch sequences and assert deterministic replay, scope rollback, and hash stability. Domain tests cover state transitions and pin/trust invariants. Filesystem tests cover links, reserved paths, alternate streams, media signatures, import expansion, atomic recovery, segment rollover, and corruption. Application tests cover preview/commit parity, CAS races, drift, conflict resolution, and consistent usage counters.

Integration tests prove eager prompts, on-demand loads, CLI-derived resources, and Settings previews receive the same effective hash. Frontend contract tests keep Tauri mapping and Web/mock behavior aligned. UI tests cover immutable base plus mutable Overlay, stale forms, trust review, reconciliation, history verification failures, responsive layout, and accessibility.

## Risks / Trade-offs

- [Exact matching creates frequent conflicts after base updates] → Make conflicts explicit, preserve lower-scope content, and provide focused reconciliation rather than risky fuzzy application.
- [System/User terminology is confused with Skill layers] → Label the UI as Overlay scope and base layer separately, and keep them separate in all contracts.
- [Append-only history grows indefinitely] → Segment at 4 MiB, paginate reads, and defer governed retention/export policy instead of deleting normal history silently.
- [Scanner false positives block legitimate security guidance] → Return safe rule ids and allow users to rewrite content; never provide a bypass that persists the matched secret or authority override.
- [Binary asset validation misses an executable format] → Require extension, media type, and magic signature agreement and maintain a deny-first executable signature table.
- [Multiple scopes make diffs hard to understand] → Return per-scope intermediate hashes and diffs, and present the complete final effective diff before every commit.
- [A failed higher scope leaves users uncertain about active content] → Surface last healthy scope and effective hash consistently in management, agent diagnostics, and UI.
- [Concurrent UI, agent, or future Curator mutations race] → Require revision and base witnesses on every write and serialize commits per Overlay key.
- [First runtime change is not implemented yet] → Keep this change implementation-blocked until `establish-effective-skill-runtime` supplies the effective snapshot, pin, usage, and logical-resource contracts.

## Migration Plan

1. Complete and verify `establish-effective-skill-runtime` before applying this change.
2. Add Overlay domain models, scanners, replay engine, and pure tests without exposing mutation commands.
3. Add storage roots, manifest and payload repositories, history segments, transaction recovery, and usage-counter integration.
4. Add read-only Overlay discovery and preview; confirm Skills with no Overlay retain byte-identical effective instructions and resource hashes.
5. Add local patch, learned-guidance, and supporting-file mutations behind CAS and pin enforcement.
6. Add import quarantine, trust promotion, base-drift detection, conflicts, reconciliation, disable, and revert.
7. Route eager prompt, on-demand load, CLI-derived mounts, previews, and resource reads through the shared replay result.
8. Add Tauri commands, shared frontend contracts, both adapters, the per-Skill Overlay UI, localization, and accessibility tests.
9. Run the full repository verification suite and strict OpenSpec validation before enabling the feature by default.

Rollback first disables new Overlay mutations, then switches consumers back to base effective packages. Overlay manifests, payloads, and history remain untouched for a future re-enable; rollback does not merge them into base files. Because storage is additive, older binaries ignore the Overlay roots. If an Overlay transaction is in progress, recovery must complete or restore it before downgrade. Re-enabling verifies history and payload hashes before replay.
