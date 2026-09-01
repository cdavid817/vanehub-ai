import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import { cn } from "../lib/utils";
import { seatDisplayName } from "../services/seat-presentation";
import type { SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";
import { useMenuList } from "../ui/actions/use-menu-list";

type SeatOption =
  | { kind: "all" }
  | { index: number; kind: "seat"; seat: SessionSeat }
  | { kind: "departed"; seat: SessionSeat };

export interface SeatAvatarGroupProps {
  /**
   * Seats that already left. Rendered inside the same popover list as everyone else, de-emphasized
   * and labelled, rather than dropped — a departed participant's terminal/log history is still
   * attributable (`SessionSeat.leftAt`'s own contract), so hiding them from the picker entirely
   * would make that history harder to find, not easier.
   */
  departedSeats: SessionSeat[];
  onSelect: (index: number | null) => void;
  roles: ExpertRole[];
  seats: SessionSeat[];
  /** Null selects every seat, which is what a newly opened tab shows. */
  selectedIndex: number | null;
  turnStatus: TurnStatus | null;
}

function seatGlyph(seat: SessionSeat, roles: ExpertRole[]): string {
  const liveRole = roles.find((role) => role.id === seat.roleId);
  return liveRole?.avatar ?? seat.roleSnapshot?.avatar ?? "🤖";
}

function seatLabel(seat: SessionSeat, roles: ExpertRole[]): string {
  const liveRole = roles.find((role) => role.id === seat.roleId);
  return liveRole?.displayName ?? seatDisplayName(seat);
}

function isCurrentSpeaker(seat: SessionSeat, turnStatus: TurnStatus | null): boolean {
  return turnStatus?.kind === "agent" && seat.seatId != null && seat.seatId === turnStatus.seatId;
}

/**
 * Trigger-plus-popover replacement for the always-visible seat tab strip: a multi-seat session no
 * longer plants a second permanent tab bar next to the Runtime Panel's own tabs (10.21).
 *
 * The popover list is `role="listbox"`/`role="option"`, not `role="tablist"`/`role="tab"`: this
 * control picks one value (which seat's data the surface below shows) rather than switching between
 * independent panels the way the Runtime Panel's own tabs do, so `listbox` is the closer ARIA match
 * now that it opens from a trigger instead of sitting inline. Keyboard roving reuses `useMenuList`
 * (`src/ui/actions/use-menu-list.ts`, already driving `ActionMenu`) instead of the tablist's own
 * `use-tab-list.ts`: that hook is Up/Down (the WAI-ARIA APG orientation for a vertically stacked
 * popover list, vs. `use-tab-list.ts`'s Left/Right for a horizontal strip) and already keeps
 * disabled items reachable by arrow keys while a no-op on `activate`, which is exactly what a
 * departed seat's row needs — `use-tab-list.ts` has no disabled-item concept to adapt for that.
 */
export function SeatAvatarGroup({ departedSeats, onSelect, roles, seats, selectedIndex, turnStatus }: SeatAvatarGroupProps) {
  const { t } = useTranslation();
  const options: SeatOption[] = [
    { kind: "all" as const },
    ...seats.map((seat, index) => ({ index, kind: "seat" as const, seat })),
    ...departedSeats.map((seat) => ({ kind: "departed" as const, seat })),
  ];
  const { activeIndex, close, handleMenuKeyDown, handleTriggerKeyDown, menuRef, open, setActiveIndex, setOpen, triggerRef } = useMenuList(options);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (open) itemRefs.current[activeIndex]?.focus();
  }, [open, activeIndex]);

  if (seats.length <= 1) return null;

  function activate(option: SeatOption) {
    if (option.kind === "departed") return;
    close();
    onSelect(option.kind === "all" ? null : option.index);
  }

  const selectedSeat = selectedIndex !== null ? (seats[selectedIndex] ?? null) : null;
  const triggerLabel = selectedSeat ? seatLabel(selectedSeat, roles) : t("session.seatSwitcher.allSeats");

  return (
    <div className="relative border-b border-border p-1.5">
      <button
        aria-expanded={open}
        aria-haspopup="listbox"
        className="ucd-focus-ring inline-flex items-center gap-1.5 rounded-md border border-border px-2 py-1 text-xs hover:bg-accent"
        onClick={() => setOpen((value) => !value)}
        onKeyDown={handleTriggerKeyDown}
        ref={triggerRef}
        type="button"
      >
        <span aria-hidden="true" className="flex -space-x-1.5">
          {seats.map((seat, index) => (
            <span
              className={cn(
                "flex h-5 w-5 items-center justify-center rounded-full border bg-background text-[11px]",
                index === selectedIndex ? "border-primary ring-2 ring-primary/30" : "border-border",
              )}
              key={seat.seatId ?? index}
            >
              {seatGlyph(seat, roles)}
            </span>
          ))}
        </span>
        <span className="max-w-24 truncate font-medium">{triggerLabel}</span>
      </button>
      {open ? (
        <div
          aria-label={t("session.seatSwitcher")}
          className="ucd-raised absolute left-0 z-40 mt-1 min-w-48 rounded-md border border-border py-1 shadow-lg"
          onKeyDown={handleMenuKeyDown}
          ref={menuRef}
          role="listbox"
        >
          {options.map((option, index) => {
            const disabled = option.kind === "departed";
            const selected = option.kind === "all" ? selectedIndex === null : option.kind === "seat" && option.index === selectedIndex;
            const speaking = option.kind !== "all" && isCurrentSpeaker(option.seat, turnStatus);
            const label = option.kind === "all" ? t("session.seatSwitcher.allSeats") : seatLabel(option.seat, roles);
            return (
              <button
                aria-disabled={disabled}
                aria-selected={selected}
                className={cn(
                  "flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs hover:bg-accent focus-visible:bg-accent focus-visible:outline-hidden",
                  selected && "bg-[hsl(var(--nav-active-soft))] text-primary",
                  disabled && "cursor-not-allowed opacity-60 hover:bg-transparent",
                )}
                key={option.kind === "all" ? "all" : (option.seat.seatId ?? index)}
                onClick={() => activate(option)}
                onFocus={() => setActiveIndex(index)}
                ref={(element) => { itemRefs.current[index] = element; }}
                role="option"
                tabIndex={index === activeIndex ? 0 : -1}
                type="button"
              >
                {option.kind === "all" ? null : <span aria-hidden="true">{seatGlyph(option.seat, roles)}</span>}
                <span className="min-w-0 flex-1 truncate">{label}</span>
                {speaking ? (
                  <span className="shrink-0 rounded-full bg-[hsl(var(--nav-active-soft))] px-1.5 py-0.5 text-[10px] font-medium text-primary">
                    {t("session.seatSpeaking")}
                  </span>
                ) : null}
                {disabled ? <span className="shrink-0 text-[10px] text-muted-foreground">{t("session.departed")}</span> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
