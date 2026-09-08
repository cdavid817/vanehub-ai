import { useEffect } from "react";

export interface WorkspaceShortcutBinding {
  /** Chord in `Mod+<key>` form. */
  shortcut: string;
  onSelect: () => void;
}

function isApplePlatform(): boolean {
  if (typeof navigator === "undefined") return false;
  return /Mac|iPhone|iPad|iPod/.test(navigator.platform ?? "") || /Mac OS X/.test(navigator.userAgent ?? "");
}

/** Human-readable chord for tooltips: the modifier is spelled the way the platform spells it. */
export function formatActivityShortcut(shortcut: string, apple = isApplePlatform()): string {
  return shortcut.replace(/^Mod\+/, apple ? "⌘" : "Ctrl+");
}

/** `aria-keyshortcuts` wants the platform-neutral key names, not the glyphs. */
export function shortcutKeyDescriptor(shortcut: string, apple = isApplePlatform()): string {
  return shortcut.replace(/^Mod\+/, apple ? "Meta+" : "Control+");
}

/**
 * Anything that consumes typed characters: a chord there is far more likely to be the user
 * editing text than asking to switch surfaces, and the composer in particular has its own
 * bindings that must not be pre-empted.
 */
export function isTextEntryTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") return true;
  if (tag === "INPUT") {
    const type = (target as HTMLInputElement).type;
    return !["button", "checkbox", "radio", "range", "submit", "reset", "file", "color"].includes(type);
  }
  return Boolean(target.closest("[data-composer], [role='textbox'], [contenteditable='true']"));
}

export function matchWorkspaceShortcut(event: KeyboardEvent, bindings: WorkspaceShortcutBinding[]): WorkspaceShortcutBinding | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return null;
  return bindings.find((binding) => binding.shortcut.replace(/^Mod\+/, "").toLowerCase() === event.key.toLowerCase()) ?? null;
}

/** Registers the chords at the shell level; text entry and the composer keep them for themselves. */
export function useWorkspaceShortcuts(bindings: WorkspaceShortcutBinding[]): void {
  useEffect(() => {
    if (typeof window === "undefined") return undefined;
    const handle = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.repeat) return;
      if (isTextEntryTarget(event.target) || isTextEntryTarget(document.activeElement)) return;
      const binding = matchWorkspaceShortcut(event, bindings);
      if (!binding) return;
      event.preventDefault();
      binding.onSelect();
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [bindings]);
}
