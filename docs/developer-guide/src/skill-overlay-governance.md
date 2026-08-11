# Skill Overlay governance

Skill Overlays customize the effective instructions and supporting resources of a Skill without editing its authoritative package. Every change remains attributable to an Overlay revision, can be previewed before commit, and can be disabled, reverted, or reconciled without copying or rewriting the base `SKILL.md`.

## Overlay scope is not a Skill layer

The effective Skill catalog first selects one base package by canonical Skill id. Package layers resolve from Project to User to Registry to System. Overlay replay starts only after that winner has been selected.

Overlay scope answers where a customization applies:

| Overlay scope | Applies to | Replay order |
| --- | --- | --- |
| System | All management contexts | First |
| User | The current user across workspaces | Second |
| Project | One canonical workspace | Last |

An Overlay may target a base package from any layer. A Project Overlay over a System package does not turn that package into a Project package, and a System Overlay does not make an immutable package editable. With no active workspace, Project Overlays are excluded.

The UI therefore reports the base layer and active Overlay scopes separately. Agent bindings continue to reference the canonical Skill id and consume only the final governed effective view.

## Storage boundaries

Overlay manifests store mutations and base witnesses, not complete package copies. Supporting payloads are content-addressed, and history is append-only.

```text
~/.vanehub/skill_overlays/<skill-id>.json
~/.vanehub/skill_overlays/user/<skill-id>.json
~/.vanehub/skill_overlays/.payloads/
~/.vanehub/skill_overlays/history/<skill-id>/

<workspace>/.vanehub/skills/.overlays/<skill-id>.json
<workspace>/.vanehub/skills/.overlays/.payloads/
<workspace>/.vanehub/skills/.overlays/history/<skill-id>/
```

System and User manifests have separate paths but share the home payload and history roots. Project state stays inside its canonical workspace. Reserved payload, history, transaction, and quarantine directories are excluded from Overlay discovery.

Callers address resources by logical paths under `references`, `templates`, or `assets`. They never receive mutable payload paths. Absolute paths, parent traversal, hidden components, reserved device names, alternate streams, links that escape the boundary, and executable or script content are rejected before persistence.

## Trust and validation

A local mutation becomes trusted only after its content, path, media, size, witnesses, and replay result pass validation. An imported ZIP v1 package is scanned in an isolated quarantine directory and then stored as untrusted. It cannot affect instructions or resources until the user reviews and promotes its exact revision and document hash.

Promotion reruns scanning, payload verification, base comparison, and replay. A changed revision, document hash, base witness, payload, scan result, or pin state makes the review stale. There is no override for hard-deny findings such as private-key material, credential structures, prompt-authority overrides, executable markup, or disguised executable content.

Validation and refusal diagnostics use the unified logging service. Logs contain safe rule ids, hashes, sizes, and redacted identities rather than submitted instruction or secret-bearing content.

## Deterministic replay and fallback

Replay operates on an immutable effective-package snapshot:

1. Verify the canonical identity, base and package hashes, trust, document revision, and payload hashes.
2. Start with the selected base instructions and resources.
3. Replay System, User, and then matching Project scopes.
4. Within a scope, apply active exact patches in creation order, append active learned-guidance blocks under one fixed delimiter, and merge resources by logical path.
5. Publish the final instructions, resource view, per-scope hashes, shadow summaries, and conflict state.

Exact patches use Unicode string equality without fuzzy matching or whitespace normalization. A non-`replace_all` patch requires exactly one match; a `replace_all` patch requires at least one match.

If one scope has a conflict or integrity failure, its entire tentative result is discarded. The runtime keeps the last healthy lower-scope result and marks dependent higher scopes blocked. Untrusted imports and unresolved drift are also excluded. Eager prompts, `load_skill`, resource reads, previews, and CLI-derived mounts all consume this same last healthy effective snapshot.

When the base instruction or package hash changes, the Overlay enters reconciliation. Even a clean replay requires the user to review the new complete diff before accepting new witnesses. A conflicting mutation can be edited or ignored; ignoring disables it while retaining its record and history.

## Pinning

Pinning freezes committed effective behavior. Healthy Overlay revisions that were active before pinning continue to replay, but every operation that could change Overlay state is refused, including create, import, promotion, patch, guidance, file changes, disable, revert, and reconciliation.

Drift remains visible while pinned. The user must explicitly unpin before resolving it. A pinned refusal does not advance the revision, append history, or update usage counters.

## Enforced limits

The default Overlay boundary enforces:

| Item | Limit |
| --- | --- |
| Combined instruction mutation text | 65,536 characters |
| Mutations in one Overlay | 256 |
| Logical resource path | 240 characters and 8 components |
| Import archive entries | 512 |
| Supporting file | 1 MiB |
| Compressed import package | 8 MiB |
| Expanded import content | 32 MiB |
| Active history segment | 4 MiB |

Imports must contain one root `overlay.json` and only its declared `payloads/sha256/<lowercase-sha256>` entries. Duplicate, missing, undeclared, encrypted, linked, traversing, hash-mismatched, size-mismatched, or unsupported entries reject the entire import.

## Atomic commit and recovery

Each mutation requires the expected Overlay revision and effective base hash. Preview and commit use the same preparation pipeline, but commit rechecks live witnesses to close the time-of-check/time-of-use gap. Stale requests do not persist partial state; the UI keeps unsaved input and requires a new preview.

Under a per-Overlay lock, one recoverable transaction stages the next manifest, reachable payloads, one linked history event, and usage-counter changes. A failure before commit restores the complete prior revision. A failure after the commit decision completes the complete next revision during recovery. The runtime never accepts a mixture of manifest, payload, history, and usage states.

History events and segments form a verification chain. A segment rolls over before 4 MiB. Missing, truncated, or tampered history is reported as an integrity failure and is never silently repaired or omitted. Revert creates a new revision and event; it does not delete the original mutation or audit record.

Operators should not edit manifests, payloads, transaction markers, or history files manually. Recovery and reconciliation must run through the Skill application service so that hashes, witnesses, history, and usage remain consistent.

## Runtime boundary

The native `tooling::skills` application service owns Overlay discovery, validation, replay, transactions, and recovery. React components use `agent-service.ts`; only the Tauri adapter invokes native commands. The Web/mock adapter models the same revisions, stale witnesses, quarantine, pinning, conflicts, history, and revert behavior without claiming native filesystem persistence.
