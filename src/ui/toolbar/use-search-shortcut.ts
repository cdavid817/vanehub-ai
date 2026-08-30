import { useEffect, type RefObject } from "react";

const EDITABLE_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT"]);

function isEditingContext(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return EDITABLE_TAGS.has(target.tagName) || target.isContentEditable;
}

/**
 * The toolbar's single keyboard entry point: `/` focuses the page-local search unless the user
 * is already typing somewhere. Distinct from a global command-palette shortcut (e.g. Cmd/Ctrl+K)
 * owned elsewhere — this is page-scoped, not a cross-destination jump.
 */
export function useSearchShortcut(targetRef: RefObject<HTMLElement | null>) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "/" || event.metaKey || event.ctrlKey || event.altKey) return;
      if (isEditingContext(event.target)) return;
      event.preventDefault();
      targetRef.current?.focus();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [targetRef]);
}
