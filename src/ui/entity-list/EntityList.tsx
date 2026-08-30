import { useRef, type ReactNode } from "react";
import { VirtualList, type VirtualListHandle } from "../virtual-list/VirtualList";

export interface EntityListProps<T> {
  items: readonly T[];
  itemKey: (item: T, index: number) => string;
  /** Undefined means nothing is active yet — arrow keys then land on the first item. */
  activeId: string | undefined;
  onActiveIdChange: (id: string) => void;
  /** Enter/Space or a click — distinct from moving the active cursor with arrow keys alone. */
  onActivate?: (item: T) => void;
  renderItem: (item: T, isActive: boolean) => ReactNode;
  estimateSize: () => number;
  ariaLabel: string;
  className?: string;
  overscan?: number;
}

function domId(key: string): string {
  return `entity-list-option-${key}`;
}

/**
 * Adds keyboard-navigable, screen-reader-visible active selection on top of `VirtualList`.
 * `aria-activedescendant` rather than per-item DOM focus, because a virtualized list's "active"
 * item may not be physically rendered — roving `tabIndex` cannot target an element that does not
 * exist yet, so the active id must be resolved to an index and scrolled into view first.
 */
export function EntityList<T>({
  items,
  itemKey,
  activeId,
  onActiveIdChange,
  onActivate,
  renderItem,
  estimateSize,
  ariaLabel,
  className,
  overscan = 8,
}: EntityListProps<T>) {
  const listRef = useRef<VirtualListHandle>(null);
  const keys = items.map((item, index) => itemKey(item, index));
  const activeIndex = activeId ? keys.indexOf(activeId) : -1;

  function moveTo(index: number) {
    if (index < 0 || index >= items.length) return;
    onActiveIdChange(keys[index]);
    listRef.current?.scrollToIndex(index);
  }

  function handleKeyDown(event: React.KeyboardEvent) {
    if (items.length === 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveTo(activeIndex < 0 ? 0 : Math.min(activeIndex + 1, items.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      moveTo(activeIndex < 0 ? items.length - 1 : Math.max(activeIndex - 1, 0));
    } else if (event.key === "Home") {
      event.preventDefault();
      moveTo(0);
    } else if (event.key === "End") {
      event.preventDefault();
      moveTo(items.length - 1);
    } else if ((event.key === "Enter" || event.key === " ") && activeIndex >= 0) {
      event.preventDefault();
      onActivate?.(items[activeIndex]);
    }
  }

  return (
    <VirtualList
      activeDescendantId={activeId ? domId(activeId) : undefined}
      ariaLabel={ariaLabel}
      className={className}
      estimateSize={estimateSize}
      getItemKey={itemKey}
      items={items}
      onKeyDown={handleKeyDown}
      overscan={overscan}
      ref={listRef}
      renderItem={(item, index) => {
        const key = keys[index];
        const isActive = key === activeId;
        return (
          <div
            aria-selected={isActive}
            id={domId(key)}
            onClick={() => { onActiveIdChange(key); onActivate?.(item); }}
            role="option"
          >
            {renderItem(item, isActive) as ReactNode}
          </div>
        );
      }}
      role="listbox"
    />
  );
}
