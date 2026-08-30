import { useEffect } from "react";

/**
 * Ctrl/Cmd+K, global — same `document`-level `keydown` pattern as the toolbar's own
 * `useSearchShortcut` (`ui/toolbar/use-search-shortcut.ts`), whose own doc comment already
 * anticipates this hook living here. Deliberately does *not* guard against an editing context the
 * way that page-local `/` shortcut does: a modifier-based combo is not something a reader ever
 * types by accident mid-sentence, and every comparable command palette (VS Code, Linear, GitHub)
 * opens from inside a text field, not just outside one.
 *
 * Alt/Shift are excluded, not just ignored: several apps bind Ctrl/Cmd+Shift+K to something else
 * entirely (Firefox's DevTools console, for one) — matching only the bare chord avoids silently
 * stealing a combination a reader already has muscle memory for elsewhere.
 */
export function useCommandCenterShortcut(onOpen: () => void) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key.toLowerCase() !== "k" || event.altKey || event.shiftKey) return;
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      onOpen();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onOpen]);
}
