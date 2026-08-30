## Context

See [proposal.md](proposal.md) for motivation and the desktop-runtime-verification delta for the required behavior. The desktop harness starts an embedded WebDriver server and launches one worker per spec. Each worker deliberately closes the native app to make the owned-process marker authoritative. On Linux, the next worker can observe the old driver port before that process finishes exiting, then fail while creating its session.

The frontend also records a generic fatal marker for browser errors and unhandled rejections. The marker currently drops the event's diagnostic detail, so a screen sweep cannot determine whether it found an application failure or test-environment noise.

## Goals / Non-Goals

**Goals:**

- Make every desktop WDIO worker start against a session-ready, test-owned driver on supported platforms.
- Keep the clean native shutdown and process-ownership checks intact.
- Include redacted browser-error context in fatal-marker evidence and failure assertions.
- Report expected `BLOCKED` cases separately from assertion failures.

**Non-Goals:**

- Changing production desktop driver behavior, production logging sinks, or Web/mock adapter behavior.
- Running credentialed live-agent, keyring-write, installer-download, or IM-bot scenarios in the release suite.
- Suppressing real frontend errors to make automation pass.

## Decisions

### Stabilize the driver at worker startup

Apply the existing embedded-driver stability probe on every supported host before a worker creates its session. It will probe the test-owned server repeatedly and restart it when shutdown has invalidated the previous process.

This preserves the current worker `after` shutdown hook. Removing that hook would hide lifecycle regressions and weaken ownership evidence; waiting for the port to close in that hook races the still-active WDIO session teardown.

The preferred delivery is an upstream-compatible dependency upgrade or a small, documented package patch. A local harness retry is the fallback only if the service exposes no safe cross-platform hook.

### Preserve diagnostics at the frontend boundary

Represent the first fatal browser event as a bounded, redacted record containing its event type and a safe message/reason. Surface the record through test-only evidence and include it in a screen-sweep assertion failure.

The marker remains a failure signal: missing detail is reported as unavailable, not treated as success. This keeps React components free of direct Tauri calls and leaves production native logging unchanged.

### Treat blocked results as evidence, not screenshots of failures

Make screenshot capture and result aggregation distinguish an expected skipped/blocked scenario from a failed assertion. Preserve the blocking reason in the run summary while reserving failure screenshots for actual failed tests.

## Risks / Trade-offs

- [A package patch diverges from the published WDIO service] → Keep it minimal, version-pinned, and remove it when an upstream release contains the fix.
- [Stability probes add worker-start latency] → Bound probes and only restart when readiness fails; this is cheaper than an entire failed desktop layer.
- [Browser reasons may contain sensitive input] → Reuse the existing redaction and length limits before evidence is written.
- [A generic fatal event may still originate outside the app] → Preserve event metadata and avoid claiming a product root cause without corroborating native or frontend evidence.

## Migration Plan

1. Add the test-only lifecycle and diagnostic behavior behind the existing desktop-e2e build path.
2. Run the affected unit/harness checks and the full desktop WDIO suite on Linux.
3. Run the repository validation commands and observe the existing cross-platform CI matrix.
4. Roll back the dependency patch or harness change if it creates a driver startup regression; production artifacts are unaffected.
