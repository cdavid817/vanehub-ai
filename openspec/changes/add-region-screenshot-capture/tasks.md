## 1. Capture domain and application lifecycle

- [ ] 1.1 Add typed run/display identities, logical and physical rectangle validation, bounded monitor snapshots, stable capture outcomes, and safe error codes in the local-media domain.
- [ ] 1.2 Define capture, overlay-window, temporary-snapshot, clock, diagnostic, and completion ports so application tests never read the real desktop.
- [ ] 1.3 Implement the single-active-run coordinator for start, commit, cancel, timeout, stale composer scope, display changes, shutdown, and exactly-once cleanup/window restoration.
- [ ] 1.4 Add domain/application tests for HiDPI conversion, negative monitor origins, cross-display clamping, invalid/stale submissions, races, limits, focus restoration, and redacted diagnostics.

## 2. Native capture and Tauri integration

- [x] 2.1 Add the maintained `xcap` dependency with bounded PNG encoding and update every Linux CI/package prerequisite block required by its official build matrix.
- [x] 2.2 Implement the real monitor capture adapter, reducing permission, compositor, no-display, and native failures to stable safe categories without shell commands.
- [x] 2.3 Implement run-owned in-memory snapshots, deterministic release, crop/re-encode, and handoff through existing OCR byte admission; require any future file-backed adapter to use the bounded local-media temporary root.
- [x] 2.4 Implement Tauri overlay window creation for every bounded monitor, opaque run/display labels, placement/scale snapshots, close interception, main-window hide/restore/focus, and application shutdown cleanup.
- [x] 2.5 Register the restricted `vanehub-capture` image protocol with webview-label/run/display authorization, generic failures, `image/png`, and `no-store` headers.
- [x] 2.6 Add thin start/commit/cancel commands and DTO mapping, register them in the supplemental command registry, and add architecture/contract tests proving components cannot invoke them directly.

## 3. Frontend service and selection surface

- [x] 3.1 Add strict screenshot availability, selection, and outcome types plus additive methods to `LocalMediaService`.
- [x] 3.2 Implement Tauri adapter commands and protocol URL mapping, and implement truthful Web adapter unavailability without using browser screen-capture APIs.
- [x] 3.3 Add a lazily loaded `region-capture` root selected by `src/main.tsx`, with an opaque frozen-image background, dim mask, crosshair, drag selection, live dimensions, minimum bounds, explicit cancel, secondary-click cancel, and `Escape` handling.
- [x] 3.4 Add accessibility/reduced-motion behavior and keyboard focus semantics while keeping the capture surface isolated from the normal application bootstrap and navigation state.
- [x] 3.5 Add component and adapter tests for pointer directions, clamping, multi-monitor tokens, cancellation paths, busy/errors, protocol URLs, and Web mode.

## 4. Composer and OCR workflow

- [x] 4.1 Refactor the OCR composer controller to share staged-source ownership/start logic between file selection and screenshot selection without changing review-before-append behavior.
- [x] 4.2 Add the fixed-size screenshot button beside OCR/microphone/speech with localized labels, tooltip, busy state, readiness/native-only guidance, stable keyboard order, and narrow-width behavior.
- [x] 4.3 Cancel screenshot selection when the composer scope changes and prove a late result cannot enter a different session.
- [x] 4.4 Add matching screenshot/capture/error copy to all five application locales, with complete Simplified Chinese and English wording and safe English fallback wording for the remaining locales.
- [ ] 4.5 Extend deterministic Playwright coverage through screenshot action, region drag, OCR review/edit/confirm/cancel, Web disabled state, session switching, and narrow Chinese/English layouts.

## 5. Desktop verification and documentation

- [ ] 5.1 Add a desktop-E2E synthetic capture activation that opens real overlay windows but supplies deterministic monitor pixels without reading or uploading the host desktop.
- [ ] 5.2 Add desktop tests for overlay placement, HiDPI geometry, Escape/secondary-click cancellation, frozen crop handoff, focus restoration, cleanup, and log redaction.
- [ ] 5.3 Run the opt-in live capture check where available and record Windows, macOS, Linux X11, and Linux Wayland independently as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` without retaining screenshot pixels.
- [x] 5.4 Update user-facing Local Media documentation to distinguish “select an image/PDF” from “capture a screen region,” including platform permission guidance and Web/CLI limitations.

## 6. Required validation and spec sync

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run build`, `npm run coverage:policy:test`, `npm run contracts:check`, and `npm run architecture:check`.
- [x] 6.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace`.
- [ ] 6.3 Run `npx playwright test`, `npm run desktop:unit:test`, the deterministic local-media Playwright suite, and applicable real desktop capture/local-media layers.
- [ ] 6.4 Run `openspec validate add-region-screenshot-capture --strict` and `openspec validate --specs --strict`, then update task checkboxes and platform-specific implementation evidence.
