import { useEffect } from "react";

/**
 * Task 12.6: a settings search result names a field, not just a page -- once the target page has
 * loaded, this finds the field's own rendered anchor element, scrolls it into view, and applies a
 * temporary highlight that never changes layout (a ring, not a size/spacing change) and never
 * persists (`onConsumed` clears the anchor from the caller's state once applied, and the highlight
 * class itself is removed after a fixed duration either way).
 *
 * `document.getElementById` rather than a ref: the target element belongs to whatever settings
 * page module `LazyFeature` just finished loading, a different component tree than this hook's
 * caller (the search results dropdown) has any reference into.
 */
const HIGHLIGHT_CLASSES = ["ring-2", "ring-primary", "ring-offset-2", "transition-shadow", "duration-700", "motion-reduce:transition-none"];
const HIGHLIGHT_DURATION_MS = 2000;
const POLL_INTERVAL_MS = 100;
const MAX_POLL_ATTEMPTS = 50; // Bounded (~5s): a page that never renders this id should not poll forever.

export function useSettingsAnchorHighlight(anchorId: string | null, onConsumed: () => void) {
  useEffect(() => {
    if (!anchorId) return undefined;
    // Narrowed once here: TypeScript's control-flow analysis does not carry the guard above into
    // `attempt`, a nested function it cannot prove is only ever called while `anchorId` is still
    // the same non-null value (it is, but not provably so from a closure's perspective).
    const resolvedAnchorId = anchorId;
    let cancelled = false;
    let pollTimer: number | undefined;
    let clearTimer: number | undefined;

    function attempt(remaining: number) {
      if (cancelled) return;
      const element = document.getElementById(resolvedAnchorId);
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "center" });
        if (typeof element.focus === "function") element.focus({ preventScroll: true });
        element.classList.add(...HIGHLIGHT_CLASSES);
        clearTimer = window.setTimeout(() => element.classList.remove(...HIGHLIGHT_CLASSES), HIGHLIGHT_DURATION_MS);
        onConsumed();
        return;
      }
      if (remaining <= 0) {
        onConsumed();
        return;
      }
      pollTimer = window.setTimeout(() => attempt(remaining - 1), POLL_INTERVAL_MS);
    }

    attempt(MAX_POLL_ATTEMPTS);
    return () => {
      cancelled = true;
      if (pollTimer !== undefined) window.clearTimeout(pollTimer);
      if (clearTimer !== undefined) window.clearTimeout(clearTimer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [anchorId]);
}
