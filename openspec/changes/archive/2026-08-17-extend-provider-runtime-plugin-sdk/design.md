## Context

`agent_runtime` already owns a domain `ProviderMetadata`/`ProviderCapabilities` model, an application `AgentProvider` trait and deterministic `ProviderRegistry`, and compatibility adapters for the five built-in CLIs. Invocation construction and output parsing are split across provider infrastructure modules, while detection/version data, permissions, cancellation, logging, and health remain implicit in surrounding runtime code. That split makes the registry useful but not yet a complete extension contract.

The roadmap requires an internal SDK and versioned manifest before any dynamic provider package. The current branch contains Sandbox/permission work for bounded Skill tools and a separate active Skill registry supply-chain proposal; neither establishes provenance, signatures, lifecycle, or quarantine for executable provider packages. Loading external providers in this change would therefore exceed the approved trust boundary.

This change affects the native desktop runtime and developer-facing documentation. It preserves the React service interface, Tauri adapter, Web/mock adapter, command DTOs, SQLite data, and UI. The owning bounded context remains `agent_runtime`; executable inventory and permission evaluation are consumed only through existing published contracts where needed.

## Goals / Non-Goals

**Goals:**

- Make one static provider adapter the sole owner of provider-specific launch, translation, parser, resume, cancellation, permission, option, usage, version, readiness, and health behavior.
- Negotiate behavior from declared capabilities and return stable classified errors for unsupported or malformed work.
- Validate a versioned, data-only manifest into domain values with strict unknown-field and security rejection.
- Provide one reusable conformance kit for all five built-ins and a fixture provider.
- Preserve partial UTF-8 and arbitrary chunk boundary correctness without unbounded buffering.
- Keep availability and diagnostics side-effect free and logs secret-safe.
- Supply reproducible contract, negative, fuzz/property, desktop fake-CLI, and structural performance evidence.

**Non-Goals:**

- Discovering, downloading, installing, updating, signing, loading, disabling, or quarantining external provider packages.
- A Marketplace, generic runner abstraction, local-model runtime, or runtime performance-budget platform from roadmap items 10–13.
- UI or frontend service changes, new Tauri commands, new SQLite tables, or changes to stable Agent ids.
- Completing the authenticated Antigravity `step_update` capture owned by `verify-antigravity-cli-live-runtime`.

## Decisions

### 1. Extend the existing `agent_runtime` contract rather than create a plugin context

The SDK remains an internal application port in `agent_runtime`, because provider invocation and generation lifecycle are already owned there. Provider-specific implementations remain infrastructure adapters. The composition root constructs the static registry and fails startup on invalid or duplicate declarations. A new bounded context would duplicate language and lifecycle ownership; putting this in `tooling::plugins` would confuse application integrations with Agent execution.

### 2. Use a cohesive provider adapter with narrow value contracts

The provider contract exposes immutable metadata, prerequisites, capabilities, option/permission schemas, invocation translation, an incremental parser factory, cancellation semantics, version-probe specification, and health classification. It does not expose raw processes, Tauri handles, SQLite connections, or logging implementations. Generic orchestration executes returned specifications through existing process/operation ports.

Provider adapters may share reviewed helpers for common JSONL framing and redaction, but provider identity matching is confined to static construction and each adapter's own implementation. Session and generic runtime modules resolve by stable id and use negotiated capabilities only.

Alternative: multiple small traits per concern. Rejected for Stage 1 because registration could accidentally assemble mismatched metadata/parser/launcher implementations. Internally the cohesive adapter delegates to small testable components, and a later ABI design can split it deliberately.

### 3. Model capability requests explicitly

Capabilities become a typed enum/query rather than booleans guessed by callers. Requests for resume, terminal, permissions, model, reasoning, usage, structured events, or cancellation are checked before preparing or running work. Unsupported requests produce a classified `unsupported-capability` error containing only provider id and capability, with no fallback.

### 4. Keep manifest parsing data-only and fail closed

`schemaVersion: 1` declares stable id/name, `runtime: cli`, reviewed executable basenames, and capability flags. Parsing denies unknown schema versions, unknown fields, duplicate keys/ids, control characters, absolute or traversing paths, separators in executable names, empty lists, inconsistent capabilities, install/update hooks, commands, arguments, environment values, scripts, URLs, dynamic-library paths, or entrypoints. A manifest validates declarations; it never constructs or registers executable code.

Built-in manifests are compiled assets or constants validated during registry construction. A test-only fixture exercises registration without entering production composition. External manifests presented to this release are classified as unsupported and cannot alter the registry.

Alternative: accept YAML plugins from disk. Rejected because a data manifest without an approved executable package provenance/ABI cannot safely bind behavior, while interpreting commands from YAML would create an unreviewed code execution boundary.

### 5. Normalize output through bounded incremental decoders

Each invocation owns a parser instance that accepts separately tagged stdout/stderr byte chunks and emits the existing runtime event vocabulary: text/thinking increments, tool lifecycle, session id, usage, completion, and classified failure. A shared bounded UTF-8/line framer retains incomplete byte and line tails, rejects or classifies oversized records, and flushes valid terminal tails. Structured JSON providers parse complete records; text fallback emits valid text without losing split code points.

Parser state prevents a completion payload from duplicating already emitted incremental content. Malformed structured events become protocol diagnostics or classified failures according to the provider contract; secrets and raw prompts never enter error display strings or unified logs.

### 6. Keep detection, cancellation, permissions, and health as specifications

Readiness and version methods return bounded executable probe specifications executed through existing process infrastructure; they do not spawn an interactive process. Cancellation declares the existing process-tree termination strategy and deadline. Permission mapping converts generic permission intent into reviewed adapter arguments after capability checks. Model/reasoning mappings validate values and create arguments without logging sensitive prompt or environment data. Health combines safe detection/version/parser outcomes into stable categories rather than raw stderr.

### 7. Make conformance executable against every provider

A reusable Rust conformance module accepts an adapter plus fixture vectors. It asserts deterministic declaration, duplicate rejection, side-effect-free readiness specs, launch/prompt mapping, cancellation, resume, unsupported capability behavior, parser chunk invariance, usage, redaction, version-failure classification, and manifest agreement. The five production adapters and fixture provider use the same suite. Property tests generate chunk partitions, including partial UTF-8; bounded deterministic corpus tests cover malformed manifests and sensitive arguments.

The fake-CLI desktop integration launches a repository fixture executable/script only through the existing test runtime, verifies streaming/session/usage/cancel behavior, and never adds it to the product registry.

### 8. Preserve frontend adapter parity by structural verification

No new frontend method or Tauri command is required. Existing contract checks and Web/Tauri tests must stay green. Playwright and visual matrices are recorded as not applicable because there is no rendered behavior change; they are still run if repository policy or an unexpected UI edit makes them applicable.

### 9. Measure deterministic structural budgets

Benchmarks measure parser throughput over fixed fixtures and registry resolution over a fixed provider set, while correctness tests enforce bounded buffer limits and linear single-pass chunk processing. Results are evidence, not a fragile shared-runner millisecond gate. Roadmap item 10's global performance budgets remain out of scope.

## Risks / Trade-offs

- [A larger trait becomes difficult to evolve] → Keep request/result value types versionable, document compatibility, and avoid a public binary ABI in Stage 1.
- [Built-in behavior changes during extraction] → Pin invocation/output fixtures and run old compatibility tests plus the common conformance suite before removing any compatibility path.
- [Manifest suggests external support that does not exist] → Document it as declaration schema only and reject external registration/loading explicitly.
- [Parser buffering enables memory abuse] → Bound undecoded bytes and record length, test oversized and adversarial partitions, and emit classified protocol failures.
- [Provider-specific branches migrate rather than disappear] → Add structural searches/tests over provider-neutral Session/runtime modules; permit stable-id selection only in composition and adapter-owned code.
- [Antigravity live event shape remains unknown] → Preserve its existing behavior and active verification change; do not invent payload fields.
- [Native fake CLI differs by OS] → Use the repository's cross-platform test launcher conventions and report only the host actually executed.

## Migration Plan

1. Add domain/application SDK value contracts and compatibility wrappers without changing behavior.
2. Move each built-in adapter behind the expanded contract and run its conformance vectors before proceeding to the next.
3. Route generic process/session integration through negotiated SDK methods and remove superseded identity branches only after compatibility tests pass.
4. Add manifest validation, fixture provider, docs, negative/property tests, fake-CLI desktop integration, and benchmark evidence.
5. Run the complete repository gates. No data migration or user action is required.

Rollback is code-only: restore the prior compatibility adapter wiring. Existing database rows, command contracts, and frontend clients remain valid because no persisted or transport schema changes.

## Open Questions

- The external provider package ABI, source/signature trust roots, sandbox lifecycle, update/quarantine policy, and marketplace distribution remain intentionally unresolved for a later security proposal.
- Antigravity incremental `step_update` semantics remain owned by its active live-verification change.
