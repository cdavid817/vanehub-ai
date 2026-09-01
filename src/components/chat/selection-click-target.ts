/**
 * Shared by `MessageItem` and `ToolUseBlock`'s `ActivityRow`: both wrap a selection affordance
 * around content they do not own (feedback/memory buttons, file links, a nested tool-call region
 * with its own separate selection). Without this check, every click on one of those would also
 * fire the wrapping selection, which is not what the click meant.
 *
 * `boundary` is the wrapper itself (the handler's own `currentTarget`). The wrapper carries
 * `role="button"` -- which is in `excludedSelector` too, so it always matches its own `closest()`
 * lookup -- so a match is only real exclusion when it names something *inside* the wrapper, not
 * the wrapper's own role.
 */
export function isInteractiveClickTarget(
  target: EventTarget | null,
  excludedSelector: string,
  boundary?: EventTarget | null,
): boolean {
  if (!(target instanceof Element)) return false;
  const match = target.closest(excludedSelector);
  return match !== null && match !== boundary;
}

/**
 * Enter/Space activates a `role="button"` wrapper the same way a click does, but only when the
 * keyboard event's own target is the wrapper itself -- otherwise a keypress already handled by a
 * focused nested control (a button, a disclosure) would also fire the wrapping selection.
 */
export function isSelfKeyActivation(event: {
  key: string;
  target: EventTarget | null;
  currentTarget: EventTarget | null;
}): boolean {
  return (event.key === "Enter" || event.key === " ") && event.target === event.currentTarget;
}
