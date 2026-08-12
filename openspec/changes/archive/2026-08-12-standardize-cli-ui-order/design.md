## Context

Settings pages currently inherit Agent order from several unrelated constants and service response arrays. Create-session has a separate local order, and existing specs encode older sequences. The change is frontend-only and must keep dynamic service-backed discovery, stable ids, and the existing React service boundary.

## Goals / Non-Goals

**Goals:**

- Give settings surfaces one deterministic built-in Agent priority.
- Give create-session its requested five-CLI priority while retaining the separate OnePiece group.
- Keep custom and future Agents visible and stable.
- Prevent order drift with focused utility and UI tests.

**Non-Goals:**

- Adding configuration support for an Agent to a settings page that does not currently support it.
- Changing registry, CLI detection, persistence, launch, or service response contracts.
- Reordering non-Agent data or changing default selections outside create-session.

## Decisions

### Centralize comparison by stable Agent id

A small frontend utility will expose the settings priority and a stable sort helper. UI pages will sort copies of service-backed arrays before rendering instead of requiring both native and Web adapters to duplicate presentation policy.

Alternative considered: reorder every backend response. Rejected because ordering is presentation context, create-session intentionally differs, and changing service responses could affect non-settings consumers.

### Preserve source order for unknown ids

The stable sort helper will assign unknown ids after recognized ids and use original indexes as the fallback. This keeps plugin/custom Agent ordering deterministic without display-name locale differences.

### Keep create-session ordering explicit

Create-session will use its own exported five-CLI priority because OnePiece remains in a separate group and its default-selection semantics differ from settings. It may reuse the stable sorting primitive but not the settings priority array.

### Update mirrored managed CLI collections

Frontend type and contract collections that directly drive settings presentation will use the requested five-CLI sequence. Their membership and stable ids remain unchanged. Agent Configuration will reorder only its supported tabs and will not add a Gemini configuration tab.

## Risks / Trade-offs

- [A settings page forgets to apply the shared ordering helper] → Cover every current service-backed Agent list and the static tab/id collections with regression tests.
- [Custom Agent order changes unexpectedly] → Use original source indexes rather than alphabetical fallback.
- [Presentation order leaks into runtime behavior] → Keep the helper in the frontend library layer and make no service, adapter, native, or persistence changes.

## Migration Plan

Deploy as a presentation-only frontend update. Rollback consists of reverting the ordering utility and consumer changes; no stored data requires migration.
