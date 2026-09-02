import { useEffect, useRef } from "react";
import { Check, ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { useMenuList } from "../ui/actions/use-menu-list";
import type { WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";

export interface WorkItemStageMenuProps {
  disabled?: boolean;
  onMove: (stage: WorkItemStage) => void;
  stage: WorkItemStage;
}

/**
 * Single "Move to…" trigger + listbox popover for a work item's stage (tasks 14.8-14.9),
 * replacing the previous permanent prev-arrow/bare-select/next-arrow trio: design.md Decision 12
 * says a card drag submits one stage mutation, the non-drag path uses a single "Move to…"
 * Menu/Listbox, and "prev/next 和 stage select 不再同时常驻" (prev/next and the stage select no
 * longer stay simultaneously resident).
 *
 * Modeled on SeatAvatarGroup (src/session-workspace/seat-avatar-group.tsx), not ActionMenu
 * (src/ui/actions/ActionMenu.tsx): this control picks one value out of a mutually exclusive set
 * with the current stage highlighted among the others, which is the `role="listbox"`/
 * `role="option"`/`aria-selected` shape. ActionMenu's `role="menu"`/`menuitem` models a list of
 * independent actions with no "currently selected" member, and ColumnVisibilityMenu's checkbox
 * list picks several values at once rather than exactly one -- neither fits this control's own
 * shape. Reuses the same `useMenuList` roving-focus hook ActionMenu and SeatAvatarGroup both
 * already share, so Arrow/Home/End/Escape keyboard navigation and outside-pointerdown-to-close
 * (PointerEvent unifies mouse/touch/pen, so this already covers a touch tap outside the popover)
 * come for free instead of being rebuilt for touch specifically.
 */
export function WorkItemStageMenu({ disabled, onMove, stage }: WorkItemStageMenuProps) {
  const { t } = useTranslation();
  const stages = [...workItemStages];
  const { activeIndex, close, handleMenuKeyDown, handleTriggerKeyDown, menuRef, open, setActiveIndex, setOpen, triggerRef } = useMenuList(stages);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (open) itemRefs.current[activeIndex]?.focus();
  }, [open, activeIndex]);

  function select(candidate: WorkItemStage) {
    close();
    // Reselecting the already-current stage is a deliberate no-op: the bare <select> this replaces
    // never fired onChange for choosing its own already-selected option, and a redundant
    // moveWorkItem call for "move to where it already is" has no server response worth reconciling.
    if (candidate !== stage) onMove(candidate);
  }

  return (
    <div className="relative min-w-0 flex-1">
      <button
        aria-expanded={open}
        aria-haspopup="listbox"
        className="ucd-focus-ring flex h-8 w-full items-center gap-1 rounded-md border border-border px-2 text-xs hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
        disabled={disabled}
        onClick={() => setOpen((value) => !value)}
        onKeyDown={handleTriggerKeyDown}
        ref={triggerRef}
        type="button"
      >
        <span className="min-w-0 flex-1 truncate text-left">{t(`todoBoard.stage.${stage}`)}</span>
        <ChevronDown aria-hidden="true" className="h-3.5 w-3.5 shrink-0" />
      </button>
      {open ? (
        <div
          aria-label={t("todoBoard.moveTo")}
          className="ucd-raised absolute left-0 z-40 mt-1 min-w-36 rounded-md border border-border py-1 shadow-lg"
          onKeyDown={handleMenuKeyDown}
          ref={menuRef}
          role="listbox"
        >
          {stages.map((candidate, index) => {
            const selected = candidate === stage;
            return (
              <button
                aria-selected={selected}
                className={cn(
                  "flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs hover:bg-accent focus-visible:bg-accent focus-visible:outline-hidden",
                  selected && "bg-[hsl(var(--nav-active-soft))] text-primary",
                )}
                key={candidate}
                onClick={() => select(candidate)}
                onFocus={() => setActiveIndex(index)}
                ref={(element) => { itemRefs.current[index] = element; }}
                role="option"
                tabIndex={index === activeIndex ? 0 : -1}
                type="button"
              >
                <span className="min-w-0 flex-1 truncate">{t(`todoBoard.stage.${candidate}`)}</span>
                {selected ? <Check aria-hidden="true" className="h-3.5 w-3.5 shrink-0" /> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
