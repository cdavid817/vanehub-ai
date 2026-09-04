## Context

See [proposal.md](proposal.md) for motivation and the delta specifications for observable behavior. The current desktop shell relies on independently sized panels and scattered surface-specific feedback; several views therefore clip, overlap, or lack a recovery action when window dimensions or session lifecycle change.

## Goals / Non-Goals

**Goals:**

- Make primary desktop workflows responsive and visibly actionable.
- Preserve session history while allowing a safe user-initiated recovery attempt.
- Keep user-guide rendering and navigation safe in both desktop and Web/mock runtime modes.
- Improve visual hierarchy by extending existing Tailwind design primitives rather than adding a component library.

**Non-Goals:**

- Replacing the session runtime or automatically retrying failed provider connections indefinitely.
- Changing updater trust, signing, or channel policy.
- Rewriting all documentation content or introducing a new markdown execution environment.

## Decisions

### Use responsive shell constraints rather than panel overlays

The workspace will use explicit grid/flex minimums, `min-w-0`, overflow containment, and breakpoint-driven auxiliary-panel collapse so the conversation owns the available center region. This is preferred over fixed positioning or shrinking all panels uniformly because it protects composer and conversation usability first.

### Recover through the existing service boundary

The UI will expose recovery only after asking the session service whether the selected session is recoverable. Tauri-specific resume work remains in the Tauri adapter/native command and Web/mock supplies deterministic equivalent states. This avoids a component calling `invoke()` or attempting to recreate provider process state itself.

### Share presentational primitives for board, goals, settings, and dialogs

Existing cards, empty states, buttons, status badges, and dialog primitives will be composed with spacing, truncation, focus, and responsive utility classes. A common visual pass reduces conflicting one-off styling without changing stored board or goal data.

### Render guides with a bounded trusted renderer and route links deliberately

Guide content will use the existing approved renderer/sanitizer path (or a small existing dependency-based correction) and distinguish internal app routes from external URLs. Unsupported or missing targets fall back to a localized error state instead of navigating to a broken 404 page.

### Treat startup and notifications as shell-level feedback

Startup loading and transient notifications belong at the application shell. They will use a high-visibility but non-blocking placement with accessible text, not a left-bottom position that obscures navigation.

## Risks / Trade-offs

- [Responsive changes alter dense desktop layouts] → Verify wide and narrow viewport snapshots and retain clear minimum widths.
- [Reconnect may be attempted for a terminal failure] → Service returns recoverability and reason before enabling the action.
- [Guide markup can contain unsafe HTML] → Preserve sanitization and never render arbitrary raw HTML.
- [Visual refinement can regress localization] → Add localized copy and test Chinese and English narrow layouts.

## Migration Plan

1. Add shared service/UI recovery and shell feedback behavior behind existing runtime adapters.
2. Update affected screens and guide routing with focused tests.
3. Run browser and desktop verification before merging.
4. Roll back surface-level styling independently if a viewport regression is found; no persisted data migration is required.
