## Why

Multi-Agent group chat cannot work across two different CLIs. A relayed turn is dispatched correctly, then fails without producing a word.

`sessions.runtime_session_id` is a single column, but a provider thread belongs to one Agent. The first seat to take a turn writes its thread id there (`service.rs:4262-4265`, keyed on `self.session_id` alone), and every later turn resumes that id regardless of which seat is speaking (`process_adapter.rs:172-179`, reading `request.session.runtime_session_id`). The seat model carries no thread id of its own (`models.rs:139-145`).

Observed on Linux with seats on claude-code and codex-cli. The codex-cli seat's first turn resumed claude-code's thread:

```
thread/resume failed: no rollout found for thread id 8b8a85cd-… (code -32600)
```

confirmed against the run's database, where that id sits in the row whose `agent_id` is claude-code. The turn ended `status: "failed"` with empty content. Two seats on the *same* Agent relay end to end, which isolates the fault to thread identity rather than to routing.

The requirement itself encodes the defect. `session-runtime-management` says the runtime persists the id "with the owning VaneHub session" and that "later CLI invocations for the same session SHALL pass that id" — true and sufficient when every session had one Agent, and wrong since seats existed.

## What Changes

- Scope provider resume metadata to a seat rather than to a session: a seat resumes only a thread its own Agent created.
- Persist a per-seat provider thread id, and key capture on the seat that owns the generation.
- Start a new provider thread when the speaking seat has no thread of its own, instead of resuming another seat's.
- Keep `sessions.runtime_session_id` as the single-seat session's thread id so existing single-Agent sessions resume exactly as before, and migrate its value onto the first seat.
- Treat a rejected resume as recoverable: record it and start a fresh thread rather than failing the turn.
- Preserve Tauri command names, request/response shapes, and Web/mock behaviour.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-runtime-management`: Provider resume metadata is stored and replayed per seat, so a seat resumes only a thread its own Agent created, and a rejected resume degrades to a new thread instead of failing the turn.
- `multi-agent-group-chat`: A seat that receives a handoff takes its turn on its own Agent's thread, so a relay across two different Agents produces a reply rather than a failed, empty turn.

## Impact

- Desktop runtime: seat model and its SQLite payload, the generation event sink that captures a reported thread id, and the generation/terminal invocation paths that choose a resume id.
- Migration: existing `sessions.runtime_session_id` values move onto the session's first seat; no historical message or session row is rewritten.
- Frontend: none. No command surface changes and the id is not presented in the UI.
- Web runtime: no externally visible behaviour change.
- No new dependencies.

## Verification

Multi-Agent handoff across two different CLIs is not covered by any automated gate that runs unattended, because it needs two installed and authenticated CLI Agents. `tests/desktop/specs/domain-multi-agent.e2e.mjs` already reports this exact failure as BLOCKED with the cause named; it becomes the acceptance check and must pass rather than skip on a host that has two CLIs.
