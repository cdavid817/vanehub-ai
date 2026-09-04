## Context

See [proposal.md](./proposal.md) for the motivation. Local Media currently stores one explicit
`pythonExecutable` per OCR, STT, and TTS profile and renders each value as an editable path field.
The renderer reaches local-media behavior through `LocalMediaService`; its Tauri implementation
owns native dialogs and IPC, while the Web implementation must remain usable without claiming
host access. Rust owns profile validation, worker launch, and the SQLite-backed profile.

The runtime deliberately does not bundle Python, install packages, download models, or fall back
to an interpreter that was not saved in the active profile. Discovery therefore has to be a
bounded settings-time inspection operation, not a new worker-launch policy. Full executable paths
are legitimate configuration data on the settings page, but they remain sensitive diagnostic
data and must not be copied into logs.

The page already separates draft edits, validation, saving, and readiness probes. The redesign
must preserve those semantics while making environment selection and incomplete setup easier to
understand. React production files remain below the 300-line limit and use Tailwind and existing
UI primitives only.

## Goals / Non-Goals

**Goals:**

- Discover a small, deterministic set of usable host Python interpreters through the native
  local-media boundary and report normalized version and compatibility metadata.
- Let each engine adopt a discovered interpreter only by editing its draft, with an additional
  explicit action to apply one candidate to all engines.
- Reorganize the page into an at-a-glance setup summary, a shared Python environment selector,
  compact engine sections, and progressively disclosed advanced fields without weakening error
  visibility or keyboard access.
- Keep native and Web behavior truthful and keep existing saved profiles compatible.

**Non-Goals:**

- Installing or upgrading Python, packages, engines, or models; downloading any artifact; or
  modifying PATH, shell profiles, virtual environments, or user files.
- Searching the whole filesystem, importing engine packages, loading models, or starting media
  workers during discovery.
- Automatically saving, silently choosing a candidate, or changing runtime fallback behavior.
- Replacing the existing per-engine Python fields with one persisted global interpreter.

## Decisions

### 1. Add a typed discovery operation to the existing local-media service boundary

`LocalMediaService` gains `discoverPythonEnvironments()`. Its result contains an availability
state, a stable failure/reason code when applicable, and ordered candidates. A candidate contains
the resolved executable path, normalized Python version, discovery source, and a typed
compatibility state with a stable reason code. The Tauri adapter invokes one thin command and
normalizes its DTO; the Web adapter returns `native_unavailable` with no fabricated candidates.
Adapter contract tests require both implementations to expose the operation.

This keeps React independent of Tauri and gives future HTTP implementations the same contract.
Returning bare strings was rejected because the UI could not distinguish incompatible, stale, or
unverified entries. Folding discovery into `getProfile()` was rejected because profile loading is
persistent state while discovery is an explicit, retryable host inspection that can fail
independently.

### 2. Implement discovery as a local-media application port with a bounded native adapter

The local-media application layer owns the use case and its domain result. A discovery port owns
platform inspection and process execution, with a concrete infrastructure adapter wired during
bootstrap. The command layer only maps the domain result to a serializable DTO.

Candidate seeds are deliberately bounded:

- all platforms inspect configured OCR, STT, and TTS executable paths plus Python names resolvable
  from the current process PATH;
- Windows may additionally use the Python launcher only to enumerate interpreter paths, after
  which every path is probed directly;
- macOS and Linux do not recursively scan common installation roots or user directories.

Every candidate is invoked directly with an argv array, never through a shell. An isolated,
site-disabled probe emits only structured interpreter identity and `sys.version_info`; execution
has a short timeout, capped output, and a maximum candidate count. Resolved paths are normalized,
deduplicated with platform-appropriate path comparison, and sorted by compatibility, descending
version, then normalized path so refreshes are stable. Broken aliases, malformed output, timeouts,
and duplicates cannot fail the entire inventory.

Compatibility is evaluated against one version policy owned by the local-media domain, rather
than inferred by importing PaddleOCR, STT, or TTS packages. Profile validation continues to verify
saved path shape, while readiness probes remain the authority for interpreter identity, package,
and model usability. This keeps discovery cheap and side-effect free without claiming that a path
alone proves the interpreter version.

Arbitrary filesystem scanning was rejected because it is slow, difficult to bound, and surprising
from a privacy perspective. Using `python --version` alone was rejected because it does not
reliably provide the resolved executable identity needed for deduplication. Reusing runtime worker
startup was rejected because discovery must not create workers or import user packages.

### 3. Preserve the persisted profile and make adoption an explicit draft operation

No database or profile schema changes are required. Selecting a candidate copies its resolved path
into exactly one engine's draft `pythonExecutable`; “apply to all” performs the three draft updates
only after an explicit click. The ordinary Save action remains the sole commit point and existing
revision-conflict handling remains unchanged. Runtime launch continues to use only the executable
saved for that engine and never substitutes another discovered candidate.

If a saved path is absent from the latest inventory, the selector displays it as “configured, not
detected” and leaves it editable. The existing native file picker remains available as the custom
path fallback through `selectProfilePath({ kind: "file" })`; discovery does not replace manual
configuration.

A single persisted Python field was rejected because engines can require different environments.
Automatically selecting the highest version was rejected because compatibility metadata cannot
prove the required packages and models exist, and an automatic draft mutation would be difficult
to distinguish from a saved change.

### 4. Centralize page orchestration in the settings hook and split presentation by purpose

`useLocalMediaSettings` owns the discovery query, manual refresh, candidate-to-draft actions, and
the existing profile/status/device queries. Discovery failure does not prevent editing or loading
the saved profile. Refresh updates only the inventory and does not overwrite draft values. Query
data is cached for the mounted settings experience but is not persisted, because the host can
change between launches.

The page is decomposed into focused components:

- a compact setup overview showing the master state, Python coverage, configuration completeness,
  unsaved changes, and runtime readiness;
- one shared Python environment panel containing refresh, candidate state, per-engine assignment,
  apply-to-all, and custom-path access;
- OCR, STT, and TTS cards whose headers summarize enabled/readiness/configuration state;
- always-visible required setup fields and collapsible advanced sections; and
- a responsive sticky action area for Save/Discard and clear dirty/saving/saved/failed feedback.

At narrow widths, grids become a single column and action groups wrap instead of horizontally
scrolling. If validation or a readiness error targets a collapsed field, its section opens before
focus moves to the field. Disclosure controls use native button semantics with `aria-expanded`
and `aria-controls`; status is not conveyed by color alone. Locale keys are added in both Chinese
and English.

Keeping three complete Python path rows inside the engine cards was rejected because it repeats
the hardest setup choice and hides cross-engine consistency. Hiding all fields behind a wizard was
rejected because experienced users still need direct, non-linear access and existing profiles may
need only one correction.

### 5. Keep diagnostics safe and verification layered

Native discovery records only allowlisted facts through unified logging: operation outcome,
candidate count, source category, duration bucket, and stable reason codes. It never logs resolved
paths, command output, environment values, or raw process errors. The resolved path is returned to
the explicit settings surface because it is the value the user is choosing, but is not included in
telemetry or persisted anywhere except the existing profile after Save.

Rust unit tests cover seed bounds, structured-output parsing, version policy, path normalization,
deduplication, deterministic ordering, timeout/output limits, and partial failure. Platform-specific
tests use fixture executables or process doubles rather than the developer's Python installation.
Frontend tests cover adapter parity, truthful Web results, refresh and draft-only selection,
apply-to-all, stale configured paths, disclosures, validation focus, responsive structure, and
Chinese/English labels. Playwright exercises the user-visible settings flow; desktop fixture tests
verify the real IPC path without invoking model inference.

## Risks / Trade-offs

- [PATH and launcher discovery cannot find every environment] → Keep editable paths and the native
  picker as first-class fallbacks, and label the inventory as detected rather than exhaustive.
- [Probing several executables can delay page setup] → Bound candidates, timeout and output,
  perform discovery asynchronously, cache it for the mounted page, and expose manual refresh.
- [A version-compatible interpreter may lack required packages] → Describe compatibility as
  Python-version compatibility only and retain per-engine readiness probes as the definitive test.
- [Platform path aliases and casing can create duplicates] → Probe for resolved executable
  identity and apply platform-aware normalization before deterministic sorting.
- [Displaying full paths can expose user names during screen sharing] → Show paths only in the
  explicit configuration panel, use wrapping/truncation with an accessible full value, and never
  include them in logs or generic error messages.
- [A sticky action area can cover content at small viewport heights] → Reserve page-bottom space,
  allow actions to wrap, and verify keyboard focus and Playwright viewport variants.

## Migration Plan

1. Add domain types, discovery port, native implementation, bootstrap wiring, command DTO, and
   command registration behind additive APIs.
2. Add the service method to Tauri, Web, deterministic test adapters, and adapter contract tests.
3. Add hook orchestration and UI components, then replace the repeated Python rows while preserving
   custom path editing and existing field identifiers where practical.
4. Add both locales and layered unit, component, Playwright, and desktop fixture coverage.
5. Ship without a database migration. Existing profiles appear as selected candidates when
   detected or as retained custom paths when not detected.

Rollback removes the additive command and new UI/service method. Profiles saved by the new UI use
the unchanged schema, so an older build can continue reading and launching them without data
conversion.
