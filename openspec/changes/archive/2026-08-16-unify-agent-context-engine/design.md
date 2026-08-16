## Context

OnePiece currently assembles provider context in `agent_runtime`, while explicit file references, retrieval/Tree-sitter indexing, LSP, memory selection, plan state, context measurement, and compaction are implemented through separate models and adapters. The new behavior crosses native generation, published cross-context contracts, persistence, service adapters, and Session UI, but it must preserve the modular-monolith dependency direction and all existing provider and compaction contracts.

The Context Engine owns proactive evidence selection for a new turn. It does not own source indexing, LSP process lifecycle, file confinement, memory storage, conversation compaction, or future Agent Run/Eval orchestration. Desktop can collect real local evidence; Web/mock must provide the same contract with deterministic in-memory fixtures and explicit mock provenance.

## Goals / Non-Goals

**Goals:**

- Produce a deterministic, explainable, budget-safe evidence set from existing sources.
- Protect explicit references and authoritative task state, merge overlaps, preserve semantic boundaries, and degrade per source.
- Project selected evidence to OnePiece without letting sources append arbitrary provider text.
- Expose a bounded, content-safe manifest through native and Web/mock service adapters and an advanced Context Inspector.
- Provide deterministic benchmark evidence for quality, efficiency, latency, deduplication, and overflow.

**Non-Goals:**

- A new bounded context, new index, full-language knowledge graph, or mandatory LLM reranker.
- Replacing compaction/optimization, rewriting every provider adapter, or implementing roadmap item 04 or later.
- Persisting candidate bodies, prompts, source code, or memory content in manifests or diagnostics.

## Decisions

### Context Engine remains inside `agent_runtime`

Selection changes the OnePiece generation request and therefore belongs to the existing `agent_runtime` generation lifecycle. Domain modules define `ContextCandidate`, `ContextBudget`, ranking/deduplication/budgeting, and `ContextEvidence`; application services define source ports and orchestration; infrastructure adapts existing published APIs and persistence. Adding a peer context was rejected because it would duplicate generation policy and require private cross-context coupling.

### Existing contexts remain authoritative for source behavior

`retrieval` owns indexed text/vector/Tree-sitter search, `code_intelligence` owns trusted LSP operations, `workspaces` owns confined inspection and Git state, sessions/agent runtime own conversation and memory contracts, and task orchestration exposes authoritative plan/task state. Context collection calls only published facades or application-owned ports. Direct repository or infrastructure imports are forbidden.

### Candidate collection is bounded and partially concurrent

The planner derives a source plan from task text, session/worktree, explicit references, active model capacity, and policy. Independent optional sources run concurrently under individual deadlines, cancellation, count/byte limits, and a total collection deadline. Explicit references and authoritative state are loaded through confined adapters. A failed, warming, unavailable, or timed-out optional source contributes a content-free source outcome and does not fail generation.

### Deterministic policy precedes any optional reranker

Version `context-engine-v1` computes an integer score from explicitness, semantic relevance, symbol relation, path proximity, freshness, authority, duplication, and estimated cost with stable tie-breaking by normalized candidate id. The first release does not call a model reranker; the contract reserves an optional bounded rerank stage whose failure must retain deterministic order.

### Fingerprint and interval merging precede budgeting

Candidates carry safe content fingerprints and normalized source/range metadata. Exact fingerprints collapse into one evidence record with combined provenance. Overlapping ranges in the same canonical file merge at complete line or symbol boundaries. Raw content never participates in logs or persisted manifests. Candidates with unsafe or unresolvable provenance are rejected before ranking.

### A versioned ledger allocates the provider budget

`ContextBudget` subtracts reserved system instructions, task, recent turns, and emergency reserve from verified model capacity. Explicit references are protected but still subject to a hard request ceiling: when protected content alone cannot fit, the engine returns a typed bounded overflow and the generation follows the existing safe path instead of silently truncating it. Other source classes have bounded shares. Code is clipped only at symbol, complete line-range, or complete tool-result boundaries. Budget arithmetic uses saturating checked values and verifies final occupancy before projection.

### Projection and manifest have separate shapes

Provider projection contains only compact source labels, path/line/symbol/type, and selected content. The inspectable manifest contains no content: ids, safe fingerprints, source kind, provenance labels, selected ranges, token estimates, score buckets/breakdown, reason codes, occupancy, policy version, source outcomes, top rejected summaries, compaction correlation, and latency buckets. Bounded manifests are stored against session/generation/turn correlation with retention and count caps; persistence failure never fails generation.

### Service and UI use one additive contract

`AgentService` gains paginated manifest-list/detail methods. Only `tauri-agent-client.ts` invokes new commands; `web-agent-client.ts` returns deterministic mock manifests. Session/OnePiece exposes an advanced Context Inspector panel from context evidence, keeping normal chat uncluttered. All labels use existing locale resources and semantic CSS tokens. Visual tests cover futuristic/minimal at desktop and narrow widths.

### Benchmarking is deterministic and content-safe

A small synthetic corpus covers definition retrieval, cross-file references, tests, explicit refs, duplicates, LSP fallback, budget pressure, and memory. Tests calculate Recall@budget, Precision@budget, useful/total evidence tokens, duplicate savings, and overflow rate. Collection/ranking timings are recorded as benchmark evidence; CI enforces structural/operation-count budgets rather than fragile wall-clock milliseconds.

## Risks / Trade-offs

- [Risk] Candidate orchestration could slow every turn. → Run only planned sources, collect concurrently with hard deadlines, retain latency buckets, and keep deterministic no-network fixtures.
- [Risk] Protected references can exceed provider capacity. → Reject with an explicit overflow outcome; never silently truncate or displace protected content.
- [Risk] Duplicate fingerprints may collide or reveal content. → Use the existing safe keyed/hash convention, combine it with canonical range identity, and never expose raw digests derived without the safe fingerprint policy.
- [Risk] LSP/retrieval availability differs by machine. → Treat every source as optional, record bounded degradation, and verify Tree-sitter/retrieval fallback.
- [Risk] Inspector metadata could leak private paths or content. → Store workspace-relative bounded paths only, redact diagnostics, and add adversarial negative tests for prompt/code/memory/credential fragments.
- [Risk] New manifest persistence adds database load. → Persist metadata only with retention/count limits and indexes for session/turn lookup; Web remains in-memory.
- [Trade-off] Deterministic ranking is less semantically rich than model reranking. → It is explainable, fast, offline, and testable; optional reranking remains future-compatible but is not implemented now.

## Migration Plan

1. Add domain/application contracts and tests without enabling provider projection.
2. Add source adapters and composition through existing public APIs.
3. Add an idempotent SQLite migration for content-free manifests and retention, with legacy databases requiring no data rewrite.
4. Enable request projection for OnePiece only after final budget verification; all other agents remain unchanged.
5. Add shared frontend contracts, adapters, inspector UI, and deterministic Web fixtures.
6. Run compatibility, negative, benchmark, UI, desktop, and full repository gates before archive.

Rollback disables Context Engine assembly and removes inspector access while leaving additive manifest tables harmless. Existing messages, compaction records, retrieval indexes, LSP configuration, and memory files remain valid.

## Open Questions

None blocking. Optional model reranking, full code graphs, and Eval Platform ingestion are explicitly deferred to later roadmap changes; this change only establishes the stable manifest contract they can consume.
