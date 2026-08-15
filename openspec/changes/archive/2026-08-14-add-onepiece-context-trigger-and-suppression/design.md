## Context

See `proposal.md` for motivation. Phase one already builds provider-neutral snapshots and computes `onepiece-context-shadow-v1`; phase two invokes the optimizer only after the fixed 60,000-character predicate. The pre-request compaction hook currently runs before the normal per-invocation snapshot, and its compatibility fallback does not report whether returning `None` meant successful compaction or a best-effort summary failure.

This phase must make the trigger decision from the exact prepared request, reuse a correlated provider usage anchor during tool continuations, keep the original request untouched on every bypass/failure path, and preserve unified content-free diagnostics. The Web/mock adapter owns only its deterministic notice simulation and is not part of the native provider loop.

## Goals / Non-Goals

**Goals:**

- Promote sufficient Token-aware evidence to the authoritative automatic trigger.
- Preserve the fixed character predicate as a deterministic fallback for unknown or unusable evidence.
- Add an explicit request boundary for suppressing automatic compaction.
- Prevent repeated low-value attempts through generation-local cooldown and failure circuit state.
- Make every compaction attempt return an unambiguous outcome so state transitions are testable.

**Non-Goals:**

- A settings page or persisted user preference for suppression.
- Session-global or SQLite-persisted cooldown/circuit state.
- A new manual compaction command.
- Evidence visualization, provider-native cache edits, or managed CLI context control.
- Changing Web/mock compaction simulation.

## Decisions

### 1. Replace shadow naming with a reusable production decision

The measurement domain will expose a `ContextCompactionDecision` with the existing reserve-and-buffer formula and a new production policy version. It remains a pure result containing `should_compact`, threshold and bounded reason. `ContextSnapshot` stores it alongside the legacy character result; diagnostics continue comparing both.

The active selector uses the Token-aware result whenever it is present, including a `false` result that suppresses legacy character compaction. Only `None` falls back to the character predicate. Complete deterministic estimates are acceptable because their quality is explicit; correlated provider usage improves later tool-loop snapshots but is not required.

**Alternative considered:** require provider-reported usage before activation. Rejected because the first request has no correlated response yet, so large session history would still be controlled by characters and known-capacity custom endpoints would gain no protection.

### 2. Build the decision snapshot inside the pre-request hook

Before applying compaction, the adapter builds the exact provider body from current turns, system context, tools and generation options, projects it through the existing wire-format projection, resolves exact-match capacity, and analyzes it with the latest usage anchor. This same snapshot is passed to the optimizer if compaction proceeds, avoiding a second divergent projection.

If projection or analysis cannot yield a Token-aware result, the selector deterministically falls back to recursive turn characters. The later normal invocation snapshot remains because it represents the actual post-compaction body and is correlated with provider usage.

**Alternative considered:** make the decision from `turns` alone. Rejected because system instructions and tool schemas consume the model context and can cause premature provider overflow even when conversation turns are small.

### 3. Carry suppression at the generation process boundary

`GenerationProcessRequest` gains a provider-neutral `AutomaticCompactionMode` with `Automatic` and `Suppressed`. Production application construction uses `Automatic`; tests and future application workflows can select `Suppressed` without reaching into the wire adapter. The field is not persisted and does not require frontend, Tauri command, or Web adapter changes in this phase.

Suppression is evaluated after evidence collection but before optimizer work, so diagnostics can explain that an eligible trigger was intentionally bypassed. It never changes the prepared context or invokes a summary model.

**Alternative considered:** infer suppression from `long_context`. Rejected because provider long-context routing and permission to mutate conversation history are different user intents.

### 4. Use deterministic generation-local control state

The provider loop creates one `AutomaticCompactionState` per generation and passes it to every pre-request compaction check. It tracks the character occupancy immediately after the last successful compaction, consecutive failed eligible attempts, and whether the circuit is open.

Cooldown policy v1 requires at least 8,192 additional recursive characters after the post-success baseline. Character growth is available for every wire format and avoids wall-clock nondeterminism. The v1 circuit opens after two consecutive eligible attempts where neither optimizer nor compatibility fallback installs a candidate. Success resets failures and establishes a new cooldown baseline. Below-trigger and explicit bypasses do not count as failures. State is dropped at generation completion.

**Alternative considered:** persist state per session or use elapsed time. Rejected because stale state can suppress an unrelated later user request, persistence adds schema/migration scope, and wall time is a poor proxy for new context.

### 5. Return typed compaction outcomes

The trigger/compaction path will return a typed outcome such as `NotEligible`, `Bypassed`, `Compacted`, `Failed`, or `TerminalFailure`. Optimizer success and compatibility success both become `Compacted`; a provider summary error becomes `Failed`; event-sink failure remains terminal. This removes the current ambiguity where `None` represents multiple outcomes and lets the state machine update only after observed results.

The compatibility function will stop re-evaluating the legacy character trigger because eligibility has already been decided from the authoritative selector. It will still require a safe old-turn boundary and use untouched original turns.

**Alternative considered:** infer success by comparing turn counts. Rejected because low-cost transformations need not reduce count and a summary could preserve an equal count while materially reducing occupancy.

### 6. Emit one bounded control decision per check

Unified logs record policy version, trigger source (`token-aware` or `character-fallback`), measurement quality, threshold/occupancy counters, legacy comparison, bypass reason, cooldown growth, consecutive failures and circuit state. Values are bounded numeric/enumerated evidence; raw content and summaries never enter the record.

The existing optimizer evidence remains responsible for plan and verifier outcomes. Control logs describe only why an attempt ran or did not run and its state transition.

## Risks / Trade-offs

- **[Risk] Local Token estimates activate too early or too late.** → Preserve quality labels, exact-match capacity, reserve/buffer headroom, legacy comparison evidence, and deterministic character fallback.
- **[Risk] A false Token-aware decision allows the character count to grow very large.** → Known capacity remains authoritative; complete recursive coverage and overflow counters are required, while unknown/incomplete evidence falls back to characters.
- **[Risk] Cooldown suppresses a genuinely urgent second compaction.** → Use context growth rather than elapsed time and a modest versioned budget; a new generation always resets state.
- **[Risk] Failed summaries repeatedly consume provider quota.** → Count a failure only when no candidate was installed and open the circuit after two consecutive failures.
- **[Risk] The new request field creates constructor churn.** → Keep the enum provider-neutral, default all production paths to `Automatic`, and cover all request fixtures at compile time.
- **[Trade-off] Suppression is controllable only at the application request boundary in this phase.** → This establishes the safe contract without prematurely adding persistence or UI; a later change can expose it through matched Tauri/Web service adapters.

## Migration Plan

1. Add pure production-decision and control-state types with unit tests while retaining the legacy character comparison.
2. Add the request-level mode with `Automatic` defaults at every production constructor.
3. Refactor compaction functions to typed outcomes, then move authoritative eligibility ahead of optimizer execution.
4. Thread the latest usage anchor and generation-local state through the initial and tool-continuation checks.
5. Add bounded diagnostics and cross-wire regression tests for Token decisions, fallback, suppression, cooldown and circuit behavior.
6. Roll back by selecting character fallback unconditionally and defaulting the request mode to `Automatic`; no persisted data migration is required.
