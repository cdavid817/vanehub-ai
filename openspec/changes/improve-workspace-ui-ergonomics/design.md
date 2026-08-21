## Context

See `proposal.md` for motivation. This change collects nine reported workspace defects that share one root theme: surfaces that were built feature-first and never revisited for scanning, for window-width robustness, or for what the user does when something goes wrong.

Three of the nine are already half-solved in the codebase and only need to be connected:

- `src/lib/session-path.ts` exports `normalizeDisplayPath()`, used by the session sidebar and the information panel but not by the work board.
- `src/settings/settings-shell.tsx` already resolves a `?section=` query parameter, so routing the Help entry to a new page needs no new routing mechanism.
- `AgentLifecycle` in `src-tauri/src/contexts/agent_runtime/domain/workflow.rs` already permits a transition out of `Failed` into `Idle` and `Starting`, and `stop_generation` already cancels a generation lease and its streaming messages. What is missing is a caller that combines them and an entry point in the UI.

## Goals / Non-Goals

**Goals:**

- Make the create-session dialog, work board, and Goal Center scannable without removing any existing control.
- Make the workspace column separation and the settings navigation highlight correct at every supported width rather than only at the developer's window size.
- Give a user whose session runtime has failed one obvious, safe action that returns the session to a usable state.
- Keep every change inside the existing service boundary, styling system, and localization contract.

**Non-Goals:**

- Changing session creation semantics, work item or goal domain rules, notification publication contracts, or recovery-report reconciliation.
- Introducing a UI component library, a state manager, or an inline-style escape hatch.
- Auto-reconnecting sessions in the background, or restarting an agent process without the user asking.
- Rendering arbitrary external documentation; the documentation page renders only the bundled README shipped with the build.

## Decisions

### 1. Separate the workspace columns with a real gutter instead of relying on paint order

The workspace grid gives the session sidebar shell `overflow-visible` so its resize handle and its overflow menu can extend past the column edge. The conversation column is the next grid item and paints an opaque `bg-background`, so anything the sidebar renders past its own column boundary is at the mercy of stacking order, and the resize handle sits at `right: -5px` directly on the seam.

The fix is structural, not cosmetic: the sidebar column reserves its own trailing gutter, the resize affordance lives inside that gutter rather than outside the column, and the sidebar shell is raised into its own stacking context so an expanded overflow menu is never painted over. The narrow-width breakpoints keep the same rule so the sidebar's rightmost content — the pinned date and the multi-Agent badge — stays fully visible when the sidebar is at its minimum width.

Widening the minimum sidebar width was considered and rejected: it costs conversation width at exactly the widths where conversation width is scarcest, and it would not fix the overflow-menu case at all.

### 2. Give the create-session dialog section weight and give seats identity

The dialog currently renders six sibling sections in one flat `grid gap-4`, so the agent-mode choice, the workspace choice, and the session name all read as equally important. The dialog gains explicit section framing with a heading and a short purpose line per section, and the sections are ordered by decision dependency: who runs the session, where it runs, what it is called.

The multi-Agent seat editor is the densest part and gains the most: each seat is numbered, shows the resolved role avatar and Agent brand icon rather than only select values, and states its reviewer constraint inline when one applies. This is presentation only — `SessionSeatAssignment` keeps its existing props, seat model, and cross-family recommendation logic, and deliberately still exposes no speaking-order control, since routing belongs to the Agents and to `@` mentions.

### 3. Route Help to a bundled documentation page rather than an external link

The Help activity entry becomes a settings destination (`?section=help`) that renders the repository README through the existing `RichMarkdown` component. The README is imported at build time with Vite's `?raw` suffix in the three shipped languages, selected from the active i18n language with the English README as the fallback.

An external browser link was rejected because it fails offline and takes the user out of the desktop client. A standalone in-app route was rejected because Help is reference material and the settings center already owns every reference destination, including About.

The page renders README content as untrusted document text: the existing `RichMarkdown` link and image handling applies unchanged, so links open externally and images route through `SafeImage`.

### 4. Recover a session by clearing its runtime, not by relaunching it

`recover_session` cancels any lingering generation lease, cancels streaming messages left in flight, and sets the session lifecycle to `Idle`. It deliberately does **not** start a new agent process: CLI generations are started per message, so an idle session accepts the user's next message normally, and silently spawning a process the user did not ask for would make an error state produce billable work.

Recovery is idempotent. Recovering an already-idle session cancels nothing and reports that nothing was cancelled, which keeps the context-menu entry safe to offer unconditionally rather than only when the session looks broken. Recovery refuses archived sessions, which already reject messages for a different reason and would be restored, not recovered.

Two entry points share one action. The failure banner is discoverable — it appears in the session workspace exactly when `lifecycleState` is `failed` — and the context-menu entry is reachable from the session list without switching sessions first. Both report their outcome through the notification system, scoped to the session.

The banner is distinct from the existing `SessionRecoveryNotice`, which reports crash-recovery reconciliation of business evidence and requires an acknowledgement decision. Runtime failure and evidence reconciliation are independent concerns and the specification already says so; merging them into one banner would make an acknowledgement look like a retry.

### 5. Move toasts to the top center

The current bottom-left placement was chosen to avoid the composer's send control and the information panel's tabs, but it covers the session sidebar, which is the surface a user is most likely to be reading when a session-creation toast arrives. Top center covers none of the three, and the band is offset far enough down to clear the top bar in both of its heights rather than sitting on its search and focus-mode controls.

Narrow viewports keep a full-width top band. Toast entry animation changes direction accordingly, and the notification center, history, scoping, and bounded lifecycle are untouched.

### 6. Fix the settings navigation clipping at its cause

The settings navigation is a CSS grid whose entries use `whitespace-nowrap`. An auto-sized grid column takes its minimum from its content, so a long localized label makes the column wider than the sidebar, and the scroll container clips the selected entry's rounded highlight on both sides.

The fix is to floor the column at `minmax(0, 1fr)` and let the label truncate with an accessible full name available on hover and to assistive technology. Shrinking the font or the sidebar padding was rejected as a workaround that would break again at the next translation.

## Risks / Trade-offs

- **README bundling grows the settings chunk.** The documentation page is already lazily loaded like every other settings page, so the README text lands in that page's chunk rather than the initial bundle. The three READMEs are text and small relative to the existing settings chunk budget.
- **README content is written for GitHub.** Relative links and badge images will not all resolve inside the client. Links remain clickable and open externally; unresolved images degrade through `SafeImage` rather than breaking the page. Rewriting the README for in-app display is out of scope for this change.
- **Recovery does not diagnose.** A session that fails for a persistent reason — a missing CLI, a bad credential — will fail again on the next message. Recovery restores a usable state; it does not claim to fix the cause. The banner therefore keeps the underlying failure message visible rather than replacing it with a success state.
- **Top-center toasts sit over the conversation header.** They are transient and dismissible, and the header is not an interactive control surface during the toast's lifetime.

## Migration Plan

No data migration. Every change is additive at the contract level: one new service method with adapters in both runtimes, one new settings page id, and one new locale key group. Existing persisted values — sidebar width, sidebar presentation mode, settings section deep links — keep their storage keys and meanings.

## Open Questions

None.
