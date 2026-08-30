import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Roving-focus open/close and Arrow/Home/End/Escape navigation for a `role="menu"` popup.
 * Disabled items stay reachable by arrow keys (only `activate` is expected to no-op for them) —
 * a keyboard user still needs to land on one to discover why it's disabled.
 */
export function useMenuList<TItem>(items: TItem[]) {
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const count = items.length;

  const close = useCallback(() => {
    setOpen(false);
    triggerRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!open) return;
    setActiveIndex(0);
    function handlePointerDown(event: PointerEvent) {
      if (menuRef.current?.contains(event.target as Node) || triggerRef.current?.contains(event.target as Node)) return;
      setOpen(false);
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open]);

  function handleTriggerKeyDown(event: React.KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setOpen(true);
    }
  }

  function handleMenuKeyDown(event: React.KeyboardEvent) {
    if (count === 0) return;
    if (event.key === "Escape") {
      event.preventDefault();
      close();
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((current) => (current + 1) % count);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) => (current - 1 + count) % count);
    } else if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(0);
    } else if (event.key === "End") {
      event.preventDefault();
      setActiveIndex(count - 1);
    }
  }

  return { open, setOpen, activeIndex, setActiveIndex, triggerRef, menuRef, close, handleTriggerKeyDown, handleMenuKeyDown };
}
