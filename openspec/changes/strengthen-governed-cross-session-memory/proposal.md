## Why

A source audit of the cross-session memory system (baseline `main@7b414a20`) confirmed correctness and safety gaps that the current specs either do not cover or cover with semantics the product has not explicitly decided:

- **Identity**: the extraction output contract and the body selector address memories by display name. The trusted layer does resolve names to `target_id + expected_revision` against the snapshot's eligible set (`memory_proposals.rs`), but v2 permits duplicate names and resolution takes the first match, so two eligible memories with one name can misroute an update; an update naming nothing eligible silently becomes a create by design, which manufactures duplicates when the target was renamed, archived, or excluded.
- **Snapshot consistency**: a CLI turn resolves its personalization snapshot at turn start for injection, then `propose_memories_from_turn` re-resolves it at turn end for extraction (`service.rs`), so one turn can run under two policies.
- **Review idempotency**: approval applies the operation to the authoritative store and only afterwards marks the candidate reviewed (`review_candidates.rs`); a crash between the two leaves a pending candidate whose retry applies again and creates a second memory.
- **Multi-Agent extraction**: the extraction gate is launch-kind only (`is_cli_kind`), so every successful multi-Agent CLI seat turn extracts independently, duplicating candidates for one collaboration.
- **Source attribution**: the bridge maps every automatic extraction to `OnePieceAutomatic` (`personalization_bridge.rs`) even though `CliAutomatic` exists, conflating the producing Agent with the extracting provider.
- **Data egress**: CLI conversation content is extracted through OnePiece's provider with no independent egress decision, no pre-redaction gate before the provider call, and no dedicated extraction profile.
- **Recall semantics**: recall currently searches the compatibility view (active + global + all-Agents), so narrowed memories are not recallable at all — a coarse fail-closed neither spec'd as final nor upgraded to governed, principal-aware recall. Whether audience becomes a logical access boundary is an undecided product ADR.
- **Round semantics**: `seat_round_id` exists and a user reply after an `@user` handoff starts a new round; memory aggregation needs its own episode boundary rather than overloading rounds.

## What Changes

- Require extraction proposals and body selection to be applied only through immutable-id plus expected-revision addressing resolved by the trusted layer against the frozen eligible set; reject ambiguous duplicate-name resolution instead of taking the first match; stop converting an unmatched update into a create.
- Freeze one personalization snapshot (policy, session mode, agent, workspace, eligible manifest, extraction entitlement) per turn and reuse it for everything the turn does, including turn-end extraction.
- Make candidate review idempotent end-to-end: reserve the resulting memory id on the candidate, record an apply operation ledger entry before mutating the authoritative store, and recover interrupted approvals without creating a second record.
- Aggregate multi-Agent automatic extraction per memory episode: seat turns accumulate evidence only, one bounded extraction runs at episode terminal state, explicit `remember` proposals stay immediate and are deduplicated against the aggregate; single-Agent OnePiece (with compaction) and single-Agent CLI (turn end) boundaries stay as they are.
- Introduce a `MemoryEpisode` aggregation boundary in sessions that groups one or more execution rounds, survives `@user` handoffs by linking the follow-up round, and is distinct from `seat_round_id`.
- Attribute automatic extraction with the producing Agent (`CliAutomatic` for CLI turns), the trigger kind, and the extracting profile as separate facts.
- Gate extraction behind a data-egress decision and a two-phase secret gate (before the provider call and before candidate persistence), with an explicit extraction profile instead of implicitly borrowing the OnePiece active profile.
- Decide recall semantics by ADR and specify the chosen option: this change proposes **governed recall** — recall shares the injection eligibility filter, the runtime injects the principal (agent, workspace, session mode), both retrieval paths run over the eligible id set, results are revalidated against the authoritative store before return, and the user's owner-level management search stays full-pool.
- Defer role-based audience and repository-level scope to a future schema v3; roles participate in ranking only. Worktrees remain distinct workspaces, stated explicitly.

## Impact

- Affected specs: `agent-cross-session-memory`, `unified-personalization-governance`, `retrieval-vector-search`, `multi-agent-group-chat`.
- Affected code (indicative): `contexts/agent_runtime` (extraction pipeline, memory proposals, turn snapshot plumbing, episode signals), `contexts/personalization` (candidate ledger, review saga, eligibility for recall), `contexts/retrieval` (governed recall filter and revalidation), `contexts/sessions` (memory episode), `bootstrap/personalization_bridge.rs` (source mapping, egress gate wiring).
- Developer docs: `docs/developer-guide/*/src/cross-session-memory.md` gains an audit-findings section referencing this change; further doc updates land with implementation.
- No behavior changes ship with this proposal itself; the decisions above take effect only as tasks are implemented.

## Capabilities

### Modified Capabilities

- `agent-cross-session-memory`: id/revision addressing for automatic proposals, frozen turn snapshot, per-boundary extraction triggers, source attribution, egress and secret gates.
- `unified-personalization-governance`: idempotent candidate apply with reserved resulting id and operation ledger; duplicate-name resolution rejection.
- `retrieval-vector-search`: governed recall (principal-scoped eligibility filter with authoritative revalidation) replacing the compatibility-view pool as the recall source.
- `multi-agent-group-chat`: memory episode aggregation boundary and evidence accumulation for seat turns.
