import { useEffect, useId, useState, type KeyboardEvent } from "react";

export function useComposerCompletionNavigation(identities: string[]) {
  const [activeIndex, setActiveIndex] = useState<number | null>(null);
  const listboxId = useId();
  const identityKey = identities.join("\u0000");

  useEffect(() => {
    setActiveIndex(null);
  }, [identityKey]);

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>, activate: (index: number) => void) {
    if (event.nativeEvent.isComposing || identities.length === 0) return false;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) => {
        if (current === null) return 0;
        const delta = event.key === "ArrowDown" ? 1 : -1;
        return Math.min(identities.length - 1, Math.max(0, current + delta));
      });
      return true;
    }
    if (event.key === "Escape" && activeIndex !== null) {
      event.preventDefault();
      setActiveIndex(null);
      return true;
    }
    if (event.key === "Enter" && !event.shiftKey && activeIndex !== null) {
      event.preventDefault();
      activate(activeIndex);
      setActiveIndex(null);
      return true;
    }
    return false;
  }

  return {
    activeIndex,
    activeOptionId: activeIndex === null ? undefined : `${listboxId}-option-${activeIndex}`,
    listboxId,
    onKeyDown,
    optionId: (index: number) => `${listboxId}-option-${index}`,
  };
}
