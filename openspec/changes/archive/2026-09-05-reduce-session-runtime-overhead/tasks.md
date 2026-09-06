## 1. Bounded Run Observation

- [x] 1.1 Add tests covering a terminal Run stopping observation, an absent Run with inactive work resolving once, an absent Run with active work continuing, and the terminal state staying rendered after observation stops
- [x] 1.2 Gate the Run status poll on Run activity, reusing one terminal-state predicate rather than adding another copy
- [x] 1.3 Pass work activity from the message row and the loop inspector so an owner still waiting for its Run keeps observing
- [x] 1.4 Keep a failed observation from being read as "no Run": it must not blank a displayed Run, and it must not end observation
- [x] 1.5 Bound consecutive failures regardless of reported activity, so an owner stuck reporting itself active cannot hold a permanent timer
- [x] 1.6 Drop the displayed Run when the owner changes, so a reused instance cannot show one owner's state under another

## 2. Streamed Persistence

- [x] 2.1 Add repository tests proving a streamed append leaves tool-use and rich-block columns untouched and preserves ordering across successive appends
- [x] 2.2 Append streamed content and thinking with a SQL append instead of a read-modify-write

## 3. Streaming Render Pacing

- [x] 3.1 Add a test proving the streaming row re-renders at a bounded rate while completed rows do not re-render
- [x] 3.2 Pace streaming updates on a monotonic clock and a plain timer, so neither a wall-clock step nor a hidden window can stall the stream
- [x] 3.3 Share the pacer with the floating assistant's subscription, which renders the same message components

## 4. Terminal Renderer

- [x] 4.1 Load the WebGL addon on demand for both terminal surfaces, falling back silently when it is unavailable
- [x] 4.2 Refit and re-report the terminal size once the renderer swap lands, since the process on the other end was told the pre-swap size

## 5. Shared Terminal-State Predicate

- [x] 5.1 Move the predicate beside `AgentRunState` and retire the six hand-written copies across the status components and the Web/mock adapters

## 6. Verification

- [x] 6.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`
- [x] 6.2 Run `cargo fmt`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`
- [x] 6.3 Run `npm run architecture:check`, `npm run native:panic:check`, `openspec validate --specs --strict`, and `openspec validate reduce-session-runtime-overhead --strict`

## Notes

The WebGL addon is imported on demand rather than statically. Bundling it put 113 KiB of renderer into the startup chunk and tripped the frontend chunk budget, which is the correct outcome: a launch that never opens a terminal should not pay for one. The terminal paints through the DOM renderer until the upgrade lands, which is also why the swap has to refit.

A runtime-context assertion over the blocking SSH adapters was implemented and then removed; the proposal records why the premise does not hold.

Measurement of the remaining polling sites, message list virtualization, and the `synchronous` pragma stay out of scope for the reasons recorded in the proposal.
