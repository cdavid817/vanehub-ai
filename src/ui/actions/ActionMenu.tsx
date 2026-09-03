import { useEffect, useRef } from "react";
import { MoreHorizontal, type LucideIcon } from "lucide-react";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { cn } from "../../lib/utils";
import { useMenuList } from "./use-menu-list";

export interface ActionMenuItem {
  id: string;
  label: string;
  icon?: LucideIcon;
  onSelect: () => void;
  tone?: "default" | "destructive";
  disabled?: boolean;
  /** Shown when disabled — a disabled action without a stated reason reads as broken, not blocked. */
  disabledReason?: string;
  /** Shown in a confirmation dialog before `onSelect` runs; omit for actions with no real consequence. */
  confirmation?: { title: string; description?: string; confirmLabel?: string };
}

export interface ActionMenuProps {
  items: ActionMenuItem[];
  triggerLabel: string;
  triggerIcon?: LucideIcon;
  className?: string;
}

export function ActionMenu({ items, triggerLabel, triggerIcon: TriggerIcon = MoreHorizontal, className }: ActionMenuProps) {
  const { open, setOpen, activeIndex, setActiveIndex, triggerRef, menuRef, close, handleTriggerKeyDown, handleMenuKeyDown } = useMenuList(items);
  const { confirm, confirmationDialog } = useConfirmation();
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (open) itemRefs.current[activeIndex]?.focus();
  }, [open, activeIndex]);

  async function activate(item: ActionMenuItem) {
    if (item.disabled) return;
    close();
    if (item.confirmation) {
      const confirmed = await confirm({
        confirmLabel: item.confirmation.confirmLabel,
        description: item.confirmation.description,
        title: item.confirmation.title,
        tone: item.tone === "destructive" ? "danger" : "default",
      });
      if (!confirmed) return;
    }
    item.onSelect();
  }

  return (
    <div className={cn("relative inline-block", className)}>
      <button
        aria-expanded={open}
        aria-haspopup="menu"
        aria-label={triggerLabel}
        className="ucd-focus-ring inline-flex min-h-11 min-w-11 items-center justify-center rounded-md hover:bg-accent"
        onClick={() => setOpen((value) => !value)}
        onKeyDown={handleTriggerKeyDown}
        ref={triggerRef}
        type="button"
      >
        <TriggerIcon aria-hidden="true" className="h-4 w-4" />
      </button>
      {open ? (
        <div
          className="ucd-raised absolute right-0 z-40 mt-1 min-w-40 rounded-md border border-border py-1 shadow-lg"
          onKeyDown={handleMenuKeyDown}
          ref={menuRef}
          role="menu"
        >
          {items.map((item, index) => {
            const reasonId = `${item.id}-reason`;
            return (
              <button
                aria-describedby={item.disabled && item.disabledReason ? reasonId : undefined}
                aria-disabled={item.disabled}
                className={cn(
                  "flex w-full flex-col items-stretch gap-0.5 px-3 py-1.5 text-left text-sm hover:bg-accent focus-visible:bg-accent focus-visible:outline-hidden",
                  item.disabled && "cursor-not-allowed opacity-60 hover:bg-transparent",
                )}
                key={item.id}
                onClick={() => void activate(item)}
                onFocus={() => setActiveIndex(index)}
                ref={(element) => { itemRefs.current[index] = element; }}
                role="menuitem"
                tabIndex={index === activeIndex ? 0 : -1}
                type="button"
              >
                <span className={cn("flex min-w-0 items-center gap-2", item.tone === "destructive" ? "text-destructive" : "text-foreground")}>
                  {item.icon ? <item.icon aria-hidden="true" className="h-4 w-4 shrink-0" /> : null}
                  <span className="min-w-0 flex-1 truncate">{item.label}</span>
                </span>
                {item.disabled && item.disabledReason ? (
                  <span className="pl-6 text-xs text-muted-foreground" id={reasonId}>{item.disabledReason}</span>
                ) : null}
              </button>
            );
          })}
        </div>
      ) : null}
      {confirmationDialog}
    </div>
  );
}
