## Context

`execute_with_code_intelligence` is `execution.rs:68-1045` — 978 physical lines in one function,
with 28 parameters and roughly forty `return` statements. It is the entire generation path for
`launch_kind = "api"` agents. `split-api-adapter-modules` moved it without touching it and recorded
that as a residual, both in its own `design.md` and in a comment on the `[ARCH-NATIVE-006]` path
budget in `src-tauri/tests/architecture.rs`.

Three facts about the surrounding code shape this change:

- The whole test surface for this function is `api_process_adapter/tests.rs`, whose `execute()`
  wrapper (`tests.rs:33-83`) calls it with a `RuntimeAgentCodeIntelligenceAdapter` over an
  unavailable responder. Seventeen tests go through that wrapper against a real local
  `TcpListener` fixture, so they exercise the actual HTTP send, the actual SSE parse, and the actual
  tool loop — not a mock of them. There is no integration test in `src-tauri/tests/` that reaches
  this function.
- `tests.rs` opens with `use super::*;`, so anything a test names must be reachable from `mod.rs`.
- The module already has seven sibling modules from `split-api-adapter-modules`, each owning one
  concern. Four of the seams below have an existing owner; only one needs a new module.

## Goals / Non-Goals

**Goals:**

- Reduce the function to a size where the tool-use loop is readable as a loop, by extracting
  fragments whose control flow can be shown equivalent rather than argued to be.
- Make the *declined* seams as legible as the extracted ones: the reason a cut is unsafe is the
  most useful thing this change can leave behind.
- Land every seam in the module that already owns its concern rather than inventing a bucket.

**Non-Goals:**

- Reaching a line target. A fragment that would need loop-carried state threaded through a
  signature is left in place regardless of how many lines it is.
- Changing any signature, event, wire format, or error text. `execute_with_code_intelligence`'s own
  28-parameter signature is untouched.
- Editing any existing test. New tests are added; none is modified, renamed, or removed.

## Decisions

### The safety argument, and how each seam is checked against it

The moves that preceded this change proved themselves with an identity: same item multiset, same
bodies, same test names. Extracting from inside a function body cannot produce that identity,
because a `return` inside the extracted fragment must become a value. So each seam is admitted only
if it satisfies **all four** of these, and the table in the next section records the check:

1. **Every exit is a `return <expr>;` that leaves the whole function.** A fragment containing a
   `continue`, a `break` of the parent's loop, or a `?` that propagates into the parent is not
   admitted. The one permitted transformation is `return X` → `return Err(X)` with a caller-side
   `Err(failure) => return failure`, which produces the same `GenerationProcessEvent` from the same
   logical point. `failed_retryable`, `failed_non_retryable` and `failed_configuration`
   (`mod.rs:560-576`) are pure constructors with no side effects, so moving *where* they are called
   from cannot be observed.
2. **No loop-carried state crosses the boundary implicitly.** Anything the fragment reads or writes
   that outlives one iteration is an explicit parameter (`&mut emitted_visible_content`), or the
   seam is declined.
3. **The fragment's text is unchanged apart from the exit rewrite and the receiver rename.** Every
   `?`, every error string, every side-effecting call and their relative order survive verbatim, so
   the diff reviews as "this block, behind a signature".
4. **The behaviour has a named test before the cut.** Where none existed, a characterization test is
   written against the un-split function and confirmed green first (see "Coverage established
   before the cut").

An exception, admitted deliberately: **`authorize_tool_call` violates rule 1** — three of its arms
end in `continue`. It is admitted anyway because those three `continue`s are the *last* statement of
their arm and the parent's replacement arm is also `continue`, so nothing is skipped or reordered;
the enum's three variants are exactly the three things the block can do. This is the only seam where
the argument is "the shapes correspond" rather than "the text is identical", and it is called out
here so a reviewer knows to read that one closely.

### The six seams, their homes, and their coverage

| # | Seam | Lines out | New home | Exits | Tests that cover it |
|---|---|---:|---|---|---|
| 1 | `resolve_endpoint` — provider config, endpoint metadata, context capacity, auth mode, credential, wire format | 99-181 | new `endpoint.rs` | 7 × `return <event>` | `execute_fails_non_retryably_when_no_model_is_configured`, `..._when_no_credential_is_stored`, `..._when_openai_compatible_agent_has_no_base_url`, `..._when_openai_compatible_base_url_is_blank`, `onepiece_missing_model_...`, `onepiece_missing_credential_...`, `onepiece_missing_endpoint_...`, **new** `an_endpoint_profile_context_window_smaller_than_the_request_fails_the_generation` |
| 2 | `resolve_generation_tool_catalog` — availability probes plus catalog, delegation, native tools, capability clear | 232-287 | `prompt.rs` | none | `execute_wires_plan_mode_and_retrieval_available_to_the_correct_resolve_tool_catalog_argument` (asserts on the wire body's `tools` array), plus every test that reaches a request |
| 3 | `resolve_generation_skill_tools` — skill-tool catalog context and resolution | 292-335 | `prompt.rs` | none | `execute_persists_a_completed_skill_tool_result_and_continues_the_plan_mode_loop` |
| 4 | `resolve_image_support` — image-input capability from profile, metadata, or catalog | 384-400 | `endpoint.rs` | none | every `execute()` test (the catalog fallback branch); the profile and metadata branches by the new endpoint-profile test |
| 5 | `analyze_round_context` — request projection into a `ContextSnapshot` | 430-455 | `invocation.rs` | none | the new endpoint-profile test, which reaches the overflow guard only through this snapshot |
| 6 | `stream_round` — the SSE read loop | 525-613 | stays in `execution.rs` | 5 × `return <event>` | `execute_skips_the_approval_prompt_for_an_allowed_shell_call`, `execute_returns_mcp_failure_...`, `execute_stops_tool_loop_immediately_when_mcp_call_cancels_generation`, **new** `a_rejected_token_event_fails_the_generation_retryably` |
| 7 | `record_tool_outcome` — the status/output/emit/push tail, duplicated at five call sites | 5 sites | stays in `execution.rs` | 1 × `return <event>` | `execute_skips_the_approval_prompt_for_an_allowed_shell_call`, `execute_returns_mcp_failure_and_continues_generation`, `remember_tool_call_is_rejected_without_persisting_when_memory_is_disabled`, **new** `a_rejected_completed_tool_use_event_fails_the_generation_retryably` |
| 8 | `authorize_tool_call` — the permission gate and approval wait | 871-953 | `interactive.rs` | 3 × `continue`, 3 × `return <event>` | `execute_skips_the_approval_prompt_for_an_allowed_shell_call` (Allow), `execute_returns_mcp_failure_...` (Ask→Approved), `execute_denied_mcp_call_...` (Ask→Denied), `execute_stops_tool_loop_immediately_...` (Cancelled), **new** `a_policy_denied_tool_call_returns_denial_data_without_executing` (Deny), **new** `an_answer_delivered_to_an_approval_wait_is_treated_as_a_denial` (Answered) |

Seams 6 and 7 stay in `execution.rs` because the tool-use loop is the only thing that has them; the
module's own doc comment is "the tool-use loop and the skill-tool dispatch it drives". Moving them
to a new module would create a file whose only reason to exist is that `execution.rs` is long.

### Why each seam lands where it does

`endpoint.rs` is new because no existing module owns "which endpoint are we calling, with what
credential, and what does it support". `invocation.rs` is accounting — it owns the *record* of an
invocation, not the decision of where to send it. Seam 1 and Seam 4 read the same three sources
(`request.endpoint_profile`, the stored metadata, the model catalog) with the same precedence, so
they belong together.

Seams 2 and 3 go to `prompt.rs`, which already owns `resolve_tool_catalog_with_code_intelligence`.
Seam 5 goes to `invocation.rs`, which already owns `record_context_snapshot`,
`estimated_input_characters` and `WireFormat` — the snapshot analysis is the thing
`record_context_snapshot` records. Seam 8 goes to `interactive.rs`, which already owns
`await_approval`, `permission_action_and_resource` and `ApprovalOutcome`; the gate is what calls all
three.

### Preserving the skill-tool catalog lease's drop order

`ResolvedSkillToolCatalog::lease` is an `Arc<dyn Any + Send + Sync>` held for the rest of the
generation in `_skill_tool_catalog_lease`. Seam 3 therefore extracts only the *body* of the
`if let Some(catalog)` block, returning `Option<ResolvedSkillToolCatalog>`; the three `let mut`
bindings stay in the parent, in their original order, so their drop order at the end of the
function is byte-for-byte what it was. Returning a struct that owned all three, or destructuring a
tuple, would reorder the drops — the lease currently drops after the generation counter and before
the key map, and nothing documents that this is irrelevant.

### Coverage established before the cut

Four behaviours a seam touches have no test today. Each gets a characterization test written and
confirmed green **against the un-split function**, in a commit that precedes the extraction:

- **No test constructs `endpoint_profile: Some(..)`.** `sample_request` sets it to `None`, and
  `FakeConfig` inherits the trait's default `active_endpoint_profile_metadata` → `Ok(None)`. So
  `endpoint_capacity` is `None` in every existing test, and Seams 1, 4 and 5 are exercised only
  along their fallback branches. The new test freezes a profile with a one-token context window and
  asserts the distinctive overflow failure, which is reachable only if profile → `ContextCapacity` →
  `ContextAnalysisService::analyze` → the guard all hold.
- **`Effect::Deny` and `ApprovalOutcome::Answered(_)` are untested.** Existing approval tests cover
  Allow, Ask→Approved, Ask→Denied and Ask→Cancelled. The policy-deny arm ("Denied by policy.") and
  the fail-closed answer arm are both inside Seam 8.
- **The `failed_retryable("Agent generation event handling failed.")` exit is untested**, at all
  eight of its occurrences: `CapturingSink` is the only `AgentProcessEventSink` in the suite and it
  never fails. Seams 6, 7 and 8 each contain one. A `RejectingSink` that refuses exactly the events
  matching a predicate makes each reachable.

These tests are additions. No existing test is edited, renamed, or removed — if one had needed
editing to accommodate a seam, that would have been evidence of a behaviour change and the seam
would have been abandoned instead.

### Seams deliberately not taken

- **The non-success HTTP response handler (476-523).** Its recovery path is
  `turns.remove(0); continue;` — it mutates the turn list and re-enters the loop *without*
  decrementing `round_trip`, so a context-recovery retry silently consumes one of the 25 permitted
  round trips. That coupling between `turns`, the loop-scoped `context_recovery_attempted` flag and
  the round-trip budget is load-bearing and invisible from inside a helper. Declined under rule 2.
- **The per-tool-call dispatch chain (648-1006) as one `execute_one_tool_call`.** Every one of its
  seven branches ends in `continue`, two of them mutate the request-scoped `images_in_request`
  counter, and the branch order is itself the dispatch precedence. A helper would need `&mut
  images_in_request`, `&mut executed`, and an enum with a variant per branch — which is the loop
  body with a signature bolted on, not a seam. Declined under rules 1 and 2.
- **The remaining setup bindings (182-230):** personalization, memory selection, system prompt,
  history, `tool_assisted_session`, timeout, HTTP client. Five sequentially dependent bindings, each
  with its own early return, sharing no concept beyond "more setup". Extracting them would produce a
  bag named after its position in the file. Declined under rule 3 — there is no boundary to preserve.
- **The two `maybe_compact_accounted` calls (355-381, 1015-1041), 27 near-identical lines each.**
  Tempting, but they differ in the value of `tool_assisted_session`, which changes between them, and
  both take `&mut turns`, `&mut request_sequence` and `&mut automatic_compaction_state`. A closure
  capturing those mutably would conflict with the loop's other uses of `turns`; a function would need
  all 23 arguments again. No reduction, only indirection. Declined.

### Verification is measured, not asserted

- The pre-change test-name list from `cargo test --lib -- --list` must be a **subset** of the
  post-change list, with the difference being exactly the new characterization tests and nothing
  else. Existing names cannot move, because moving one means a test was edited.
- `git diff` on `tests.rs` must contain only additions.
- Every extracted fragment is diffed against its pre-extraction text to confirm rules 1 and 3.
- The `[ARCH-NATIVE-006]` budget for `execution.rs` is lowered to its measured value; the residual
  comment is rewritten with the new function size.

## Risks / Trade-offs

- **A seam changes behaviour in a path no test reaches** → the reason four behaviours get
  characterization tests first. Where a path still has no test after that (for example, a
  `finish_api_invocation` call on the SSE read-error branch), the fragment's text is unchanged, so
  the risk is confined to the exit rewrite, which the compiler checks the type of and the diff shows
  in full.
- **`authorize_tool_call`'s enum drops a case** → the `match` in the parent is exhaustive over three
  variants, and the helper's own `match effect` is exhaustive over `Effect`. A missed case fails to
  compile. Two new tests cover the two arms that had none.
- **Seam 3 reorders the catalog lease's drop** → prevented structurally by keeping the three
  bindings in the parent, as above.
- **The subtree budget rises** → it will: new tests, one new module, eight signatures and eight call
  sites. Each component is counted and stated when the budget is raised, so a number that does not
  add up is visible rather than absorbed.
- **The function is still large afterwards** → yes. It goes from 978 lines to roughly 580. Four
  further seams were available on a line count and refused on the reasons above. A 580-line function
  that behaves identically is the better outcome.

## Migration Plan

No deployment or data migration; this is a compile-time reorganization and a revert is a plain
`git revert`. The work commits in three stages, each leaving `cargo test` green: the characterization
tests first (against the un-split function), then the no-early-exit seams (2, 3, 4, 5), then the
seams with exits (1, 6, 7, 8). Stopping after any stage leaves a coherent, smaller function.
