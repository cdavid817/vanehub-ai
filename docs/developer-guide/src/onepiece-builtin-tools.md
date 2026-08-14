# OnePiece built-in tools operations guide

OnePiece is the only Agent allowed to discover or invoke the extended native tool set. Custom API Agents and CLI-wrapped Agents cannot acquire these capabilities by copying display names, provider metadata, or capability tags. Desktop policy is authoritative; React visibility is informational.

## Rollout gates

Set a gate to the exact value `1` before launching VaneHub. Missing values, `0`, `true`, and other strings remain disabled. Gates are independent so rollback of one domain does not disable legacy file, search, shell, Skill, LSP, MCP, or unrelated OnePiece behavior.

| Environment variable | Capability | Default | Promotion criteria | Rollback trigger |
| --- | --- | --- | --- | --- |
| `VANEHUB_ONEPIECE_ARTIFACT_READ_ENABLED` | Artifact list, metadata, bounded read, and review | Enabled | Database migration, integrity, retention, and preview tests pass | Integrity mismatch, path disclosure, or unbounded preview |
| `VANEHUB_ONEPIECE_BROWSER_ENABLED` | Managed Playwright Browser read/effects and handoff | Disabled | Sidecar compatibility, navigation policy, cleanup, and handoff E2E pass | Sidecar orphan, policy bypass, profile leakage, or handoff ownership failure |
| `VANEHUB_ONEPIECE_WEB_ENABLED` | DuckDuckGo search and guarded fetch | Disabled | Provider fixture, SSRF/redirect, expansion limit, and provenance tests pass | Provider drift, address-policy bypass, credential inheritance, or unbounded response |
| `VANEHUB_ONEPIECE_CODE_EXECUTION_ENABLED` | Independent code sandbox | Disabled | Platform isolation, offline network, process-tree, quota, and cleanup tests pass | Isolation witness loss, host file access, network access, or orphan process |
| `VANEHUB_ONEPIECE_OCR_ENABLED` | Local PaddleOCR extraction | Disabled | Managed worker/PDFium checksums, compatibility, limits, and privacy tests pass | Remote fallback, protocol drift, checksum failure, or private-content log leak |
| `VANEHUB_ONEPIECE_ARTIFACT_PUBLISH_ENABLED` | Authenticated Artifact publication | Disabled | Once-only acknowledgement, hash binding, expiry, and access-control tests pass | Hash mismatch, acknowledgement bypass, or visibility/expiry failure |
| `VANEHUB_ONEPIECE_ARTIFACT_DOWNLOAD_ENABLED` | Controlled desktop download | Disabled | Hash verification, owned save path, size limit, and active-content handling pass | Source-path exposure, overwrite, hash mismatch, or unsafe file activation |
| `VANEHUB_ONEPIECE_DELEGATION_ANALYZE_ENABLED` | Claude Code/Codex CLI analysis | Disabled | Passive readiness, protocol fixtures, redaction, quotas, and cleanup pass | Credential/transcript leak, protocol drift, retry loop, or child-process escape |
| `VANEHUB_ONEPIECE_DELEGATION_EDIT_ENABLED` | Isolated delegated edits and ChangeSet sealing | Disabled | Analyze criteria plus independent workspace, offline child commands, and complete ChangeSet verification pass | Target mutation, incomplete evidence, unsealed output, or isolation loss |
| `VANEHUB_ONEPIECE_DELEGATION_APPLY_ENABLED` | Exact once-only ChangeSet apply | Disabled | Clean-base preflight, exclusive lease, rollback capsule, exact verification, crash recovery, and replay tests pass | Partial apply, stale approval, lock loss, rollback uncertainty, or recovery regression |

Rollback means remove only the affected environment variable and restart the desktop runtime. Additive database migrations and retained evidence are not deleted. In-flight owned work is cancelled and reaped; a recovery-required apply remains visible for manual inspection.

## Dependencies and readiness

- Run `npm install` to install the pinned Playwright package, then provision its managed Chromium with `npx playwright install chromium`. The native sidecar uses an isolated ephemeral context and never imports the user's normal browser profile.
- Install and enable PaddleOCR through Settings → Extensions. OCR readiness requires a managed PaddleOCR 3.x inference protocol and checksum-verified PDFium renderer. Do not hand-place binaries into the application-data directory.
- Install Claude Code or Codex CLI through their vendor-supported installer and make the executable discoverable through the normal process environment. Readiness uses passive version/help/authentication checks and does not consume model quota.
- The sandbox accepts only reviewed Python 3.11–3.14 and Node.js 20–24 runtimes. It does not install packages and never falls back to ordinary shell execution.

The OnePiece configuration page shows each capability and mode separately. Stable reasons include `disabled`, `backend_unavailable`, `policy_unavailable`, version/dependency failures, and isolation failures. Readiness checks do not open a browser, run user code, OCR content, or start an external AI task.

## Permissions and data boundaries

Arbitrary code execution, effectful Browser actions, retained downloads, delegation start, Artifact publication, and ChangeSet application require unified permission evaluation. ChangeSet application always uses a non-rememberable, once-only approval bound to Artifact id/content hash, diff hash, repository identity, exact base commit, and clean-state witness.

Artifacts are immutable logical records backed by content-addressed blobs. Tool-to-tool binary transfer uses Artifact ids rather than arbitrary paths. Publication exposes an Artifact only through VaneHub's authenticated boundary; it does not create a public Internet URL. Retention preserves referenced Artifacts and removes expired, unreferenced blobs through the governed cleanup path.

Durable logs contain bounded identifiers, outcome codes, hashes, counts, and timing only. Credentials, authorization headers, prompts, page/file bodies, OCR text, hidden reasoning, provider transcripts, raw subprocess output, and private paths are removed before the unified logging service persists an event.

## Recovery and troubleshooting

For Browser failures, confirm the gate, Node/Playwright installation, managed Chromium availability, and sidecar protocol version. A crashed sidecar is restarted at most once; repeated failure is terminal.

For Web failures, distinguish `provider_protocol_changed` from URL-policy rejection. Private, loopback, link-local, metadata, credential-bearing, and non-HTTP(S) targets are intentionally blocked at every redirect.

For sandbox or OCR failures, use the readiness reason rather than bypassing isolation or checksums. Missing proof makes the capability unavailable; there is no shell or remote-OCR fallback.

For delegation, verify the reviewed CLI version, required flags, authentication, target workspace identity, and edit-isolation support. Provider claims are informational until host evidence agrees. Failed or cancelled attempts are not retried automatically.

For ChangeSet apply, stop automatic mutation when the UI reports manual recovery. Inspect the retained recovery reference and safe instructions. Never stash, reset, merge, rebase, cherry-pick, commit, push, resolve conflicts, or partially apply files to make an automated attempt continue.

## Web/mock runtime

Web/mock implements the same TypeScript service contract with deterministic records marked `simulated`. Native effects return `desktop_runtime_required`; the mock does not claim that a browser, network fetch, sandbox, OCR worker, local publication, external CLI, or repository mutation occurred.
