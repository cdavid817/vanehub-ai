## Context

See `proposal.md` for motivation. The existing service contract already exposes structured projection health and the complete rebuild lifecycle for both desktop and Web/mock adapters. The maintenance UI previously treated those responses as opaque records, rendered only the last completion time, and drove a rebuild in one uninterrupted component callback.

The change must keep React behind the agent-service boundary, preserve the read-only nature of System Activity, remain locale-aware, and avoid changes to native persistence or authoritative evolution records.

## Goals / Non-Goals

**Goals:**

- Give operators actionable, safe per-domain projection diagnostics and recent rebuild evidence.
- Keep long rebuilds responsive by exposing phase and item progress between bounded service calls.
- Provide cancellation without allowing concurrent starts or compromising the active projection generation.
- Keep desktop and Web/mock behavior aligned through their existing shared service contract.

**Non-Goals:**

- Changing projection algorithms, rebuild budgets, source retention, or generation activation rules.
- Adding raw source payload inspection or mutation controls to System Activity.
- Adding native commands, adapter methods, dependencies, or a new persistence schema.

## Decisions

### Use explicit shared health types

Model domain and rebuild health records explicitly instead of retaining `Record<string, unknown>`. This makes every field rendered by the UI part of the checked service contract and prevents unchecked property access. The alternative was component-local parsing, which would duplicate contract knowledge and weaken adapter parity.

### Separate health presentation from the activity view

Render diagnostics in a dedicated, read-only health panel. This keeps the main view below the production line limit, gives the diagnostic hierarchy stable accessible semantics, and prevents maintenance detail from complicating timeline rendering. The alternative was expanding the existing inline section, which would make the parent view harder to test and maintain.

### Isolate rebuild orchestration in a hook

Keep the multi-phase rebuild state machine in a focused hook that invokes only existing agent-service methods. A zero-delay yield between bounded advance calls allows React to paint progress and accept cancellation, while refs retain the active rebuild identity and cancellation intent across asynchronous steps. The alternative was leaving the loop inside the controls component, where progress could not be presented cleanly and cancellation races were harder to contain.

### Treat cancellation as terminal UI state

Once cancellation is requested, the hook stops issuing validation or activation calls, delegates cancellation to the service boundary, refreshes health, and clears local progress. The service remains authoritative for preserving the previous valid generation. Duplicate starts are disabled whenever local rebuild progress exists.

### Render safe codes and localized framing

Domain ids, gap codes, failure codes, rebuild ids, and status values are already safe structured diagnostics. The UI presents those codes within localized labels, truncates long identities visually while preserving titles, and never renders opaque cursor contents or source payloads.

## Risks / Trade-offs

- [Cancellation can race with an already-dispatched bounded advance call] → Record cancellation intent immediately, avoid subsequent phases, and rely on the idempotent service cancellation contract.
- [A very large item budget can make small progress changes visually subtle] → Show exact processed and budget values in addition to the native progress element.
- [Backend status codes are not translated] → Preserve safe codes for diagnosis while localizing all surrounding labels and phases; status-code localization can be added without changing the contract.
- [Only a subset of rebuild history is visible] → Bound the panel to the three most recent records so maintenance detail does not overwhelm the activity timeline.

## Migration Plan

This is an additive frontend presentation change over existing service data. Deploy the typed records, health panel, rebuild hook, localized copy, and tests together. Rollback consists of reverting the UI and type refinements; no data migration or native rollback is required.
