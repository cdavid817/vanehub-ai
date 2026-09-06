# Design: strengthen-governed-cross-session-memory

Distilled from the full audit-and-design study (`vanehub-ai-cross-session-memory-detailed-design.md`, baseline `main@7b414a20`). "Current implementation", "current spec", and "target design" are kept distinct throughout; nothing below is shipped behavior until its tasks land.

## Architecture decisions

### ADR-001: one authoritative memory store

Keep the single host-level store with scope, audience, status, policy, and runtime eligibility. No per-Agent or per-seat physical stores: splitting creates duplication, sync conflicts, unclear ownership for multi-role Agents, and multiplies derived indexes.

### ADR-002: automatic paths only create candidates

| Source | Default result |
| --- | --- |
| Explicit user creation / explicit "save as long-term memory" | Active |
| OnePiece automatic extraction | Candidate |
| CLI proxy extraction | Candidate |
| Multi-Agent episode aggregate extraction | Candidate |
| Agent `remember` tool | Candidate |
| External file edit | Reconciliation/quarantine flow |

An Agent must never overwrite an Active memory without review.

### ADR-003: immutable id + revision is the only mutation address

Every update/archive/merge carries `target_memory_id + expected_revision`. Names are display attributes; even a unique name is not an address. The trusted layer keeps resolving the model's name references, but resolution that matches more than one eligible memory is **rejected** (a counted rejection), and an update that matches nothing is **rejected rather than downgraded to create** — the current downgrade manufactures duplicates exactly when the target was renamed, archived, or policy-excluded.

### ADR-004: one frozen governance snapshot per turn

Resolved at turn start: policy, session mode, agent, workspace, runtime capability, eligible memory manifest, extraction entitlement. Everything in the turn — injection, body selection, and turn-end extraction — reuses that snapshot or an immutable entitlement derived from it. A mid-turn policy edit affects the next turn only. (Current gap: the CLI path re-resolves at turn end.)

### ADR-005: multi-Agent extraction aggregates per memory episode

- Single-Agent OnePiece: extract with compaction (unchanged).
- Single-Agent CLI: extract at successful turn end (unchanged).
- Multi-Agent: seat turns accumulate evidence only; one bounded extraction runs at episode terminal state; explicit `remember` proposals remain immediate and the aggregate deduplicates against them.

**Execution round vs memory episode**: `seat_round_id` already exists and a user reply after `@user` handoff starts a *new* round. Episodes are the aggregation boundary: one episode spans one or more rounds, pauses at `@user`, and links the follow-up round. The two identifiers must not be merged.

```text
Session
└── Memory Episode E1
    ├── Execution Round R1  (Agent A → Agent B → @user handoff)
    ├── user reply
    └── Execution Round R2  (Agent B → Agent C → @user done)
```

### ADR-006: roles rank, roles do not gate (yet)

Phase one: Agent + workspace decide eligibility; role, responsibility, and handoff context decide ranking only. Role audience waits for schema v3 with migration, old-reader refusal, and UI semantics. Seat ids are session-local instances and never become long-term audience values.

### ADR-007: recall becomes governed (option B)

Two options were weighed:

- **A. Legacy shared recall** — keep pool-wide search, rename audience in UI to "auto-injection audience", never market it as confidentiality.
- **B. Governed recall (chosen)** — recall shares injection eligibility; the runtime injects the principal (agent, workspace, session mode); FTS and vector search run over the eligible id set; results are revalidated against the authoritative record before return; the user's owner-level management search stays full-pool.

Note the current implementation is a third state the specs never chose: recall searches the *compatibility view* (active + global + all-Agents), so narrowed memories are not recallable even by their audience. Option B subsumes this fail-closed behavior with correct per-principal semantics. Governed recall is an in-app logical boundary, not cryptographic isolation from the OS user or other local processes.

### ADR-008: worktrees stay isolated; repository scope arrives explicitly later

Different worktrees resolve to different workspaces today, deliberately. A future `Repository` scope (v3) may hold build conventions and architecture principles, with explicit promotion — never silent inheritance.

## Target pipeline

```text
Turn start ──► frozen snapshot (policy/mode/agent/workspace/eligible manifest/entitlement)
   │
   ├─ injection + body selection (id+revision addressed, from the snapshot)
   │
   ├─ single-Agent: extraction at existing boundary ──┐
   ├─ multi-Agent seat: evidence only ─► episode ledger┤
   │                          episode terminal ───────┤
   │                                                  ▼
   │                        egress gate (extraction profile, data classification)
   │                        pre-provider secret redaction
   │                        bounded structured extraction (names in, resolved to id+revision)
   │                        pre-persistence secret gate
   │                                                  ▼
   │                                          candidate queue
   │                                                  ▼
   │              review ─► idempotent apply saga (reserved resulting id + operation ledger)
   │                                                  ▼
   └─ recall(query, limit) ─► governed eligibility ─► FTS/vector over eligible ids ─► authoritative revalidation
```

## Reliability mechanics

- **Apply saga**: a create candidate reserves `resulting_memory_id` at submission; approval writes an operation-ledger entry, applies, then marks reviewed; startup recovery completes or rolls forward interrupted approvals. Retrying an interrupted approval must yield the same memory id, never a second record.
- **Extraction jobs**: persisted job/attempt records with lease, bounded retry, and input hash so app shutdown, provider timeouts, and duplicate event delivery stay idempotent.
- **Source attribution**: `source_agent` (who produced the conversation), `trigger_kind` (compaction / turn end / episode terminal / explicit tool), and `extractor_profile` (which provider profile ran the extraction) are recorded separately; the current unconditional `OnePieceAutomatic` mapping is corrected to `CliAutomatic` where the producer is a CLI turn.
- **Derived-index revalidation**: injection and recall re-check authoritative status and revision before returning content, so repair windows cannot resurface archived or deleted records.

## Non-goals

Not stored: hidden reasoning, raw terminal output, secrets, per-retry noise, facts trivially re-derivable from the repository, CLI-native private memory files, unconfirmed Agent guesses. Cross-session memory is a local-host capability — no cross-device sync. Fixed scoring weights, expiry/importance/confidence field expansions, and role audience are all deferred until evaluated (see ADR-006, phases 5–6 in tasks).
