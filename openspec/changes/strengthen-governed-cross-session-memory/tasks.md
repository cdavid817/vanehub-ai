## 1. Specification gate and terminology

- [ ] 1.1 Ratify the recall ADR (governed recall, option B) and the audience semantics it implies; record the decision in this change before any recall code moves.
- [ ] 1.2 Define execution round vs memory episode in the sessions and group-chat specs, including `@user` handoff creating a new round linked to the same episode.
- [ ] 1.3 State worktree-as-workspace isolation explicitly and record repository scope as deferred v3 work.
- [ ] 1.4 Confirm the developer-guide audit-findings section matches the final decisions.

## 2. P0 correctness fixes

- [ ] 2.1 Extend the extraction manifest entries with stable ids and pinned revisions so the model's name references resolve deterministically; reject ambiguous duplicate-name matches as counted rejections with a test over two eligible same-name memories.
- [ ] 2.2 Stop converting an unmatched update into a create; reject the action and count it, with tests covering renamed, archived, and policy-excluded targets.
- [ ] 2.3 Return `{id, revision}` from body selection instead of names; drop hallucinated ids the manifest never showed.
- [ ] 2.4 Freeze the CLI turn's personalization snapshot at turn start and reuse it in `propose_memories_from_turn`; add a test that a mid-turn policy edit does not change the turn's extraction entitlement.
- [ ] 2.5 Map CLI-produced automatic extractions to `CliAutomatic` and record `trigger_kind` and `extractor_profile` separately; migrate nothing retroactively.
- [ ] 2.6 Reserve `resulting_memory_id` on create candidates, add the apply operation ledger, apply-then-mark within a recoverable saga, and add a crash-recovery test proving an interrupted approval retried yields one memory.

## 3. Extraction platform

- [ ] 3.1 Persist extraction jobs with lease, bounded retry, and input hash; recovery after shutdown or provider timeout must be idempotent.
- [ ] 3.2 Introduce an explicit extraction profile and a data-egress decision before any provider call carries CLI conversation content.
- [ ] 3.3 Enforce the two-phase secret gate: redact or refuse before the provider call, and again before candidate persistence.
- [ ] 3.4 Surface extraction health and backpressure in the existing health UI without logging conversation content.

## 4. Multi-Agent episodes

- [ ] 4.1 Add `MemoryEpisode` to sessions, reusing `seat_round_id` and linking post-handoff rounds to the originating episode.
- [ ] 4.2 Record per-seat evidence during episode lifetime; suppress per-seat CLI automatic extraction while an episode is open.
- [ ] 4.3 Run one bounded aggregate extraction at episode terminal state, deduplicating against explicit `remember` candidates from the same episode.
- [ ] 4.4 Make surfaced-body deduplication key on session + surface principal + id + revision so one seat's view does not suppress another's.

## 5. Governed recall

- [ ] 5.1 Inject the runtime principal (agent, workspace, session mode) into recall; run FTS and vector retrieval over the eligibility-filtered id set.
- [ ] 5.2 Revalidate every hit against the authoritative record (status and revision) before return.
- [ ] 5.3 Keep the user's owner-level management search full-pool and separate from Agent recall.
- [ ] 5.4 Shadow-evaluate governed recall against the current compatibility-view behavior before removing the legacy path.

## 6. Deferred evaluations (no code until ratified)

- [ ] 6.1 Role-aware ranking through the context engine's versioned budgeting, with per-role evaluation sets and no eligibility widening.
- [ ] 6.2 Schema v3 assessment: role audience, repository scope, v2→v3 migration with old-reader refusal, worktree→repository promotion.
- [ ] 6.3 Extraction quality metrics: precision/recall, candidate approval rate, duplicate/conflict rate, injected-but-unused rate, stale-misleading rate.
