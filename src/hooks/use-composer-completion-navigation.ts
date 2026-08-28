import { useId, useState, type KeyboardEvent } from "react";

/**
 * Which completion option the keyboard is on, held by the option's identity rather than by its
 * position in the list.
 *
 * The list is rebuilt whenever its inputs change, and two of those inputs arrive asynchronously:
 * the file search that a mention token starts, and the seat roster. Holding the highlight by index
 * meant every rebuild had to clear it, so a rebuild landing between the arrow key and Enter turned
 * the Enter that was meant to accept a mention into the one that sends the message — the composer
 * emptied and the half-written line went out. Resolving the identity on each render keeps the
 * highlight on the same option across a rebuild and drops it only when that option is really gone,
 * which removes the race rather than narrowing it.
 */
export function useComposerCompletionNavigation(identities: string[]) {
  const [activeIdentity, setActiveIdentity] = useState<string | null>(null);
  const listboxId = useId();
  const resolved = activeIdentity === null ? -1 : identities.indexOf(activeIdentity);
  const activeIndex = resolved === -1 ? null : resolved;

  function step(delta: number) {
    // Resolved inside the updater rather than from this render's `activeIndex`, so two arrow
    // presses in one batch move two places instead of both moving from the same start.
    setActiveIdentity((current) => {
      const at = current === null ? -1 : identities.indexOf(current);
      if (at === -1) return identities[0] ?? null;
      const next = Math.min(identities.length - 1, Math.max(0, at + delta));
      return identities[next] ?? null;
    });
  }

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>, activate: (index: number) => void) {
    if (event.nativeEvent.isComposing || identities.length === 0) return false;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      step(event.key === "ArrowDown" ? 1 : -1);
      return true;
    }
    if (event.key === "Escape" && activeIndex !== null) {
      event.preventDefault();
      setActiveIdentity(null);
      return true;
    }
    if (event.key === "Enter" && !event.shiftKey && activeIndex !== null) {
      event.preventDefault();
      activate(activeIndex);
      setActiveIdentity(null);
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
