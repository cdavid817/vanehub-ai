import { useEffect, useRef } from "react";

export interface UseFocusTrapOptions {
  onClose: () => void;
  closeDisabled?: boolean;
  returnFocus?: HTMLElement | null;
}

/**
 * Extracted from `ApplicationDialog`'s original inline effect (see git history) so `Sheet` does
 * not reimplement Tab-cycling, Escape, nested-modal-stack awareness, and focus-return — both now
 * share one implementation. Callers attach the returned ref to their `role="dialog"` root and set
 * `aria-modal="true"` themselves; this hook only manages focus behavior, not markup.
 */
export function useFocusTrap<T extends HTMLElement>({ onClose, closeDisabled = false, returnFocus }: UseFocusTrapOptions) {
  const rootRef = useRef<T>(null);
  const closeRef = useRef(onClose);
  const closeDisabledRef = useRef(closeDisabled);
  closeRef.current = onClose;
  closeDisabledRef.current = closeDisabled;

  useEffect(() => {
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const root = rootRef.current;
    // A descendant's own mount effect — e.g. content requesting a specific field — runs before
    // this one in the same commit (children's effects fire before their parent's), so a focus it
    // already placed inside this root wins over the generic fallback below.
    if (!root?.contains(previousFocus)) {
      const focusTarget = root?.querySelector<HTMLElement>("[data-dialog-autofocus]") ?? root;
      focusTarget?.focus();
    }

    function handleKeyDown(event: KeyboardEvent) {
      const modalStack = Array.from(document.querySelectorAll<HTMLElement>('[aria-modal="true"]'));
      if (modalStack.at(-1) !== root) return;
      if (event.key === "Escape" && !closeDisabledRef.current) closeRef.current();
      if (event.key !== "Tab" || !root) return;
      const focusable = Array.from(root.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])',
      )).filter((element) => !element.hidden && element.getAttribute("aria-hidden") !== "true");
      if (focusable.length === 0) {
        event.preventDefault();
        root.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      (returnFocus ?? previousFocus)?.focus();
    };
  }, [returnFocus]);

  return rootRef;
}
