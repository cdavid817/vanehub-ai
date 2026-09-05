## 1. Typed Health Presentation

- [x] 1.1 Replace opaque projection health records with explicit shared domain and rebuild health types.
- [x] 1.2 Add a localized, read-only health panel for lease, domain backlog, cursor, failure, gap, and recent rebuild diagnostics.
- [x] 1.3 Integrate the health panel into System Activity without exposing source payloads or mutation controls.

## 2. Responsive Rebuild Maintenance

- [x] 2.1 Extract rebuild orchestration into a hook that reports bounded processing, validation, activation, and catch-up phases.
- [x] 2.2 Prevent duplicate rebuild starts and add service-boundary cancellation that preserves the previous projection.
- [x] 2.3 Add localized progress, cancellation, and completion feedback to the maintenance controls.

## 3. Verification

- [x] 3.1 Add component tests for per-domain health diagnostics, recent rebuild history, progress, and cancellation.
- [x] 3.2 Run frontend lint, tests, build, and relevant desktop verification for the shared UI behavior.
- [x] 3.3 Run Rust workspace checks for native compatibility and validate the OpenSpec change and main specs in strict mode.
