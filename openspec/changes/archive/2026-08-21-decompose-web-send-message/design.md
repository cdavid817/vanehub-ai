## Context

See proposal.md — Why. `sendMessage` currently performs three different jobs in one 470-line method: it admits and records a turn synchronously, derives deterministic simulation inputs, and registers the timers that publish the Web/mock stream. The existing chat state module already owns messages, subscriptions, and active timers, while `web-chat-client.ts` owns the rest of `ChatMessagingService`. The hard 300-line limit prevents moving the method intact.

The public service interface, Tauri adapter, React callers, and Rust/native layer are unaffected. Browser mode must preserve the current deterministic simulation; native desktop mode continues through `tauri-agent-client.ts` without any textual change.

## Goals / Non-Goals

**Goals:**

- Preserve synchronous validation and state mutation before `sendMessage` resolves.
- Preserve every scheduled event's condition, payload, delay, and registration order.
- Keep all timer identifiers in the single active-stream record so cancellation remains complete.
- Move `sendMessage` under the existing `webChatClient` composition seam while keeping every production TypeScript file at or below 300 lines.

**Non-Goals:**

- Changing mock responses, timing constants, policy behavior, memory extraction, skill binding, tool approval, MCP simulation, or compaction behavior.
- Changing the `ChatMessagingService` or `AgentService` signature.
- Sharing the Web/mock scheduler with the native adapter or introducing a generic event framework.
- Refactoring adjacent service methods beyond imports and composition cleanup required by the move.

## Decisions

### Separate turn admission from event scheduling with an explicit immutable context

`sendMessage` remains the orchestration entry point in `web-chat-client.ts`. It performs the existing validation, runner/run creation, message insertion, and session update in the same order, then constructs a context containing the session, input, normalized config, effective policy, messages, tokens, and settings-derived flags. Scheduling modules receive that context and append timer ids to one caller-owned array.

This keeps state writes visible at the same time as today and prevents helpers from re-reading mutable session state after scheduling. Passing an explicit context is preferred over a module-level mutable "current turn" because concurrent sessions must not share orchestration state.

### Split schedulers by behavior cluster, not by timer count

Two focused modules hold the interleaved callbacks:

- response scheduling: started, compaction, memory extraction/injection, skill evidence, token, thinking, generic tool use, rich blocks, and completion;
- API-tool scheduling: shell approval, clarification, plan exit, grep, explicit memory, and MCP approval.

Each scheduler preserves the source block order and original numeric delays. The orchestrator calls them at the same points as the original method so equal-delay timers retain registration order. This is preferred over one helper per timeout, which would inflate interfaces and obscure ordering, and over a single scheduler file, which would still exceed 300 lines.

### Characterization tests observe externally visible invariants

Focused fake-timer tests record emitted events and assert the ordering of the existing milestones, admission failures, immediate run/session/message state, completion, and cancellation. Feature-specific payload assertions already present in `web-agent-client.test.ts` remain unchanged. Tests use the public `webAgentClient` surface and existing test seams rather than exporting scheduler internals.

This provides evidence at the service boundary and avoids tests that merely mirror helper structure.

### Preserve composition and contract ownership

`sendMessage` moves into the existing `webChatClient: ChatMessagingService`; `webAgentClient` keeps a single `...webChatClient` spread and loses its inline method plus obsolete imports. No signature moves because `ChatMessagingService` already owns it. `webAgentClient: AgentService` remains the compile-time conformance check.

### Close both changes only after the full verification set passes

After focused checks, run the repository-required frontend and full validation commands. Only then mark this change complete and tick task 3.4 in `extract-web-client-state-modules`. Ratchets use measured line counts; no new ESLint exemption is added.

## Risks / Trade-offs

- **Equal-delay callbacks change order across helper calls** → Keep source registration order explicit in the orchestrator and assert the ordered event sequence with fake timers.
- **A timer is omitted from cancellation bookkeeping** → All schedulers append to the same `timeoutIds` array before `setWebActiveStream`; cancellation tests advance timers afterward and assert no later events arrive.
- **A helper re-reads state and observes later mutations** → Compute the immutable turn context synchronously and pass values into schedulers.
- **New module boilerplate raises the services aggregate** → Move blocks rather than duplicate them, measure the subtree, and ratchet the existing architecture budget only to the justified result.
- **The refactor passes focused tests but breaks another adapter contract** → Keep the Tauri file byte-identical and run type, contract, unit, build, architecture, Playwright, and strict OpenSpec validation checks.

## Migration Plan

Land characterization tests first, then extract the scheduling clusters, move the orchestrator, and finally ratchet budgets and update both task files. Each boundary must pass TypeScript, contract, and focused tests. Rollback is a normal commit revert because there is no persisted-data or deployment migration.
