## 1. Tool identity

- [x] 1.1 Define the `exit_plan_mode` tool with a bounded `plan` argument, and offer it from the plan-mode catalog only.
- [x] 1.2 Register it so plan-mode catalog resolution and the existing-tool registry agree it exists.
- [x] 1.3 Classify it for permission evaluation without granting it any resource-modifying action.

## 2. The blocked request

- [x] 2.1 Handle it inline in the tool loop, where the event sink and blocked-call channel are, as the question already is.
- [x] 2.2 Validate the plan argument before publishing anything, rejecting an empty or over-long plan with a message the model can act on.
- [x] 2.3 Publish it as `awaiting_input` and block until the decision arrives.
- [x] 2.4 Refuse in a non-interactive execution context rather than blocking on a user who is not there.
- [x] 2.5 Return approval, decline, and cancellation as distinct results, with approval stating that write tools apply from the next turn.

## 3. Delivering the decision

- [x] 3.1 Add a command that delivers approve/decline over the existing blocked-tool-call transport and records no permission grant.
- [x] 3.2 Expose it through the service boundary in both the desktop and web clients.
- [x] 3.3 Render the proposed plan on the chat surface with approve and decline actions.
- [x] 3.4 Move the session from plan to execute on approval, only once the decision has reached a live waiter.
- [x] 3.5 Leave the session in plan mode on decline.

## 4. Tests

- [x] 4.1 The tool is offered in plan mode and absent from the ordinary catalog.
- [x] 4.2 An approved request returns success and states that the change applies to the next turn.
- [x] 4.3 A declined request returns a distinct, non-approving result.
- [x] 4.4 A non-interactive context refuses without blocking.
- [x] 4.5 An empty or over-long plan is rejected before anything is published.
- [x] 4.6 The mode changes only after the decision reaches a live waiter, and not on decline.
- [x] 4.7 The decision writes no permission record.
- [x] 4.8 Component coverage for the approve and decline actions on the chat surface.

## 5. Validation

- [x] 5.1 `npm run lint:ci`
- [x] 5.2 `npm run test`
- [x] 5.3 `npm run build`
- [x] 5.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 5.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 5.6 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 5.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 5.8 `npx playwright test`
- [x] 5.9 `openspec validate add-agent-plan-exit-request --strict`
- [x] 5.10 `openspec validate --specs --strict`

## Status

`exit_plan_mode` publishes the model's plan to the chat surface, blocks on the user, and moves the
session out of plan mode when they approve. It is offered from the plan-mode catalog only, so a
session that is not planning never carries its schema.

Two things the survey settled before any code was written. The approval had to be its own tool
rather than a phrasing of `ask_user_question`, because an answer is a string the model interprets
and nothing about it would move the session out of plan mode. And it takes effect on the next turn:
the catalog and policy are resolved once per generation, so the requesting turn genuinely does not
have the write tools, and the success message says so rather than letting the model discover it by
calling a tool it was never given.

D4 changed during implementation and the design records both versions. The plan was for the approve
button to invoke a callback that set the mode, which meant drilling one boolean through five
presentational components. The backend already publishes the same signal -- an approved request
resolves its tool block to `completed`, a declined one to `failed`, and it only emits that block if
the blocked generation was alive to receive the decision. Reading the decision off the transcript
gives the identical "only when a live waiter received it" guarantee from the component that
actually knows, works whichever surface approved it, and survives the approving card unmounting.

Two machine-enforced rules shaped the final code rather than review comments: `tests/architecture.rs`
rejected the `if approved { Approved } else { Denied }` mapping inside the Tauri command, which now
lives in the API layer; and clippy required the same `result_large_err` allowance its sibling
`ask_user_question` already carries.
