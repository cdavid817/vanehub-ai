import { useId, useState, type KeyboardEvent } from "react";

type CompletionSelection = {
  activeIndex: number | null;
  identityKey: string;
};

export function useComposerCompletionNavigation(identities: string[]) {
  const listboxId = useId();
  const identityKey = identities.join("\u0000");
  const [selection, setSelection] = useState<CompletionSelection>({ activeIndex: null, identityKey });
  const activeIndex = selection.identityKey === identityKey ? selection.activeIndex : null;

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>, activate: (index: number) => void) {
    if (event.nativeEvent.isComposing || identities.length === 0) return false;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setSelection((current) => {
        const currentIndex = current.identityKey === identityKey ? current.activeIndex : null;
        if (currentIndex === null) return { activeIndex: 0, identityKey };
        const delta = event.key === "ArrowDown" ? 1 : -1;
        return {
          activeIndex: Math.min(identities.length - 1, Math.max(0, currentIndex + delta)),
          identityKey,
        };
      });
      return true;
    }
    if (event.key === "Escape" && activeIndex !== null) {
      event.preventDefault();
      setSelection({ activeIndex: null, identityKey });
      return true;
    }
    if (event.key === "Enter" && !event.shiftKey && activeIndex !== null) {
      event.preventDefault();
      activate(activeIndex);
      setSelection({ activeIndex: null, identityKey });
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
