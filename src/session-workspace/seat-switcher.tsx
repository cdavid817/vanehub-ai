import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { ExpertRole } from "../types/expert-role";
import type { SessionSeat } from "../types/agent";

/**
 * Selects whose terminal, shell, or log stream a seat-scoped tab is showing.
 *
 * This lives inside the tab rather than adding tabs, because multiplying nine tabs by the seat count
 * is unusable at three seats. Renders nothing for a single seat, so those sessions keep the
 * interface they already have.
 */
export function SeatSwitcher({
  onSelect,
  roles,
  seats,
  selectedIndex,
}: {
  onSelect: (index: number) => void;
  roles: ExpertRole[];
  seats: SessionSeat[];
  selectedIndex: number;
}) {
  const { t } = useTranslation();
  if (seats.length <= 1) return null;

  return (
    <div aria-label={t("session.seatSwitcher")} className="flex flex-wrap gap-1 border-b border-border p-1.5" role="tablist">
      {seats.map((seat, index) => {
        const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
        const selected = index === selectedIndex;
        return (
          <button
            aria-selected={selected}
            className={cn(
              "ucd-interactive flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs",
              selected ? "border-primary bg-[hsl(var(--nav-active-soft))]" : "border-transparent",
            )}
            key={index}
            onClick={() => onSelect(index)}
            role="tab"
            type="button"
          >
            <span aria-hidden="true">{role?.avatar ?? "🤖"}</span>
            <span className="truncate">{role?.displayName ?? seat.agentId}</span>
          </button>
        );
      })}
    </div>
  );
}
