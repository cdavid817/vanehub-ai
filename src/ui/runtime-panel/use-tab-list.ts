import { useRef } from "react";

/** Roving-tabindex Left/Right/Home/End navigation for a horizontal `role="tablist"`. */
export function useTabList<TTab extends { id: string }>(tabs: TTab[], activeTabId: string, onActiveTabChange: (id: string) => void) {
  const tabRefs = useRef(new Map<string, HTMLButtonElement>());

  function focusAndActivate(id: string) {
    onActiveTabChange(id);
    tabRefs.current.get(id)?.focus();
  }

  function handleKeyDown(event: React.KeyboardEvent) {
    if (tabs.length === 0) return;
    const currentIndex = tabs.findIndex((tab) => tab.id === activeTabId);
    if (event.key === "ArrowRight") {
      event.preventDefault();
      focusAndActivate(tabs[(currentIndex + 1) % tabs.length].id);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      focusAndActivate(tabs[(currentIndex - 1 + tabs.length) % tabs.length].id);
    } else if (event.key === "Home") {
      event.preventDefault();
      focusAndActivate(tabs[0].id);
    } else if (event.key === "End") {
      event.preventDefault();
      focusAndActivate(tabs[tabs.length - 1].id);
    }
  }

  function registerTabRef(id: string) {
    return (element: HTMLButtonElement | null) => {
      if (element) tabRefs.current.set(id, element);
      else tabRefs.current.delete(id);
    };
  }

  return { handleKeyDown, registerTabRef };
}
