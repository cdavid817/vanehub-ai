## 1. Context Measurement Domain

- [x] 1.1 Add provider-neutral context component, API-round, semantic class, retention class, measurement quality, capacity, snapshot, usage-anchor, and shadow-decision domain types with bounded values and stable version fields.
- [x] 1.2 Add API-round grouping tests covering plain conversation, consecutive assistant responses, multiple tools in one response, missing results, orphan results, duplicate identifiers, and provider-normalized resumed history.
- [x] 1.3 Implement deterministic ordered classification rules and tests for protected control context, current user intent, user corrections, recent verbatim rounds, older summarizable rounds, reclaimable tool output, reinjectable state, and conservative unknown content.
- [x] 1.4 Add aggregate snapshot invariants ensuring every projected component belongs to exactly one semantic category, every conversational component belongs to one round, and no protocol-incomplete round receives a removable retention class.

## 2. Estimation and Model Capacity

- [x] 2.1 Implement the versioned deterministic local estimator for Unicode text, structured message envelopes, system instructions, tool schemas, supported non-text blocks, and character-only degradation.
- [x] 2.2 Add multilingual and structural estimator fixtures that verify determinism, monotonic growth, bounded arithmetic, and conservative handling of unknown blocks without asserting equality with a provider tokenizer.
- [x] 2.3 Define the versioned embedded model-context catalog schema with exact stable provider/model keys, context-window Tokens, optional verified maximum output, metadata revision, and safe source identity.
- [x] 2.4 Add catalog validation tests for unique exact keys, positive safe bounds, deterministic ordering, required revisions, and rejection of display-name or fuzzy aliases.
- [x] 2.5 Populate the initial catalog only from current official provider metadata, recording a reviewable source identity and revision for each entry and leaving ambiguous or custom models unknown.
- [x] 2.6 Thread stable source provider identity into `ApiProviderConfig` for preset-derived profiles while preserving unknown identity for custom profiles, and add configuration mapping regression tests.
- [x] 2.7 Implement capacity calculation and `onepiece-context-shadow-v1` decision tests for known capacity, unknown capacity, Token estimates, characters-only measurements, boundary equality, and overflow-safe reserve arithmetic.

## 3. Prepared Request Projection

- [x] 3.1 Refactor request preparation so the exact immutable provider request body is built once, analyzed, and then sent without analysis rebuilding or mutating it.
- [x] 3.2 Add an Anthropic pure projection from recorded request fixtures into neutral components and protocol-safe rounds, including top-level system instructions and declared tools.
- [x] 3.3 Add an OpenAI-compatible pure projection from recorded request fixtures into the same neutral vocabulary, including injected system messages, function tools, assistant tool calls, and tool results.
- [x] 3.4 Add cross-wire contract tests proving semantically equivalent Anthropic and OpenAI-compatible requests produce equivalent categories, retention classes, round completeness, and aggregate character coverage.
- [x] 3.5 Verify each projection's aggregate character walk covers the complete prepared request and fails conservatively when an unknown provider-native block is encountered.

## 4. Analysis Service and Usage Anchors

- [x] 4.1 Add a focused `ContextAnalysisService` that orchestrates projection, estimation, grouping, classification, capacity resolution, active character-decision comparison, and the non-mutating shadow decision.
- [x] 4.2 Implement generation-local request fingerprints and content fingerprints without retaining raw message, prompt, tool argument, tool result, credential, header, or protocol-frame content.
- [x] 4.3 Finalize a sent request's snapshot from valid normalized provider input usage and create a same-generation `ContextUsageAnchor` correlated to provider, model, request fingerprint, and invocation sequence.
- [x] 4.4 Reconcile identical and append-only successor requests as `reported` or `reported-plus-estimated-delta`, with tests for appended tool rounds and unchanged request retries.
- [x] 4.5 Invalidate anchors and fall back to full estimation for changed system instructions, tool schemas, provider, model, component order, removed prefixes, malformed usage, degenerate zero usage, or broken invocation sequencing.
- [x] 4.6 Add property and boundary tests ensuring snapshot failure never changes the prepared request and all analysis failures degrade to existing compaction behavior.

## 5. Runtime Integration and Diagnostics

- [x] 5.1 Invoke context analysis before the initial OnePiece provider request and correlate the snapshot with session, operation, generation, and request sequence.
- [x] 5.2 Invoke context analysis before every tool-continuation request and thread the generation-local usage anchor through the bounded tool loop.
- [x] 5.3 Finalize analysis anchors from Anthropic and OpenAI-compatible normalized usage without duplicating, relabeling, or changing invocation-grained token-accounting observations.
- [x] 5.4 Add bounded unified-log projection for measurement quality, capacity, component and class totals, policy versions, active and shadow decisions, disagreement reasons, safe hashes, and overflow counts.
- [x] 5.5 Add logging tests proving diagnostics omit prompts, messages, tool inputs and outputs, credentials, request headers, raw frames, and raw provider payloads.
- [x] 5.6 Preserve `maybe_compact_accounted` as the only active trigger and add regression tests proving shadow disagreements cannot trigger, suppress, or alter compaction and summary output.
- [x] 5.7 Confirm snapshot, projection, estimator, anchor, and comparison failures are best-effort diagnostics and cannot fail, delay indefinitely, or mutate the owning generation.

## 6. Compatibility and Documentation

- [x] 6.1 Update agent-runtime architecture documentation with the analysis boundary, measurement-quality meanings, exact-match capacity catalog, generation-local anchor lifecycle, and shadow-mode non-authority.
- [x] 6.2 Add regression coverage for the existing 60,000-character threshold, six-turn retention, summarization-only call, memory-extraction interaction, visible compaction notice, and internal `ContextCompaction` usage accounting.
- [x] 6.3 Verify custom and discovered model ids absent from the capacity catalog remain usable, report unknown capacity, and continue under existing character-count behavior.
- [x] 6.4 Verify managed CLI agents and the Web/mock adapter remain behaviorally unchanged because this phase adds no frontend service contract or user-visible projection.
- [x] 6.5 Run `npm run contracts:check` after provider catalog or shared-contract changes and resolve all contract drift without bypassing repository checks.

## 7. Required Verification

- [x] 7.1 Run `npm run lint:ci` and resolve all findings.
- [x] 7.2 Run `npm run test` and `npm run test:coverage`, including the new context-analysis regression suites, and satisfy coverage policy.
- [x] 7.3 Run `npm run build` and verify no frontend or shared TypeScript contract regression.
- [x] 7.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and resolve all formatting differences.
- [x] 7.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and resolve all warnings.
- [x] 7.6 Run `cargo test --manifest-path src-tauri/Cargo.toml` and verify both wire formats, accounting, compaction, and context-analysis tests pass.
- [x] 7.7 Run `cargo check --manifest-path src-tauri/Cargo.toml` successfully.
- [x] 7.8 Run `npm run coverage:policy:test` and `npm run version:unit:test` successfully.
- [x] 7.9 Run `openspec validate add-onepiece-context-measurement-and-classification --strict` and `openspec validate --specs --strict` successfully.
