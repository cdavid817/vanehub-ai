import { Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentWithModelFamily } from "../services/agent-model-family";
import { addSeat, removeSeat } from "../services/seat-mutation";
import type { ExpertRole } from "../types/expert-role";
import type { SessionSeat } from "../types/agent";

export type SeatActivity = "idle" | "speaking";

/**
 * Shows who is in the session and lets the line-up change while it runs, because the collaboration
 * path emerges during the work.
 *
 * Deliberately absent: any control that picks who speaks next. Routing belongs to the Agents and to
 * `@` mentions; offering a dispatch button here would quietly turn the human into a supervisor.
 */
export function SessionSeatsPanel({
  activity,
  agents,
  onSeatsChange,
  roles,
  seats,
}: {
  activity: Record<number, SeatActivity>;
  agents: AgentWithModelFamily[];
  onSeatsChange: (seats: SessionSeat[]) => void;
  roles: ExpertRole[];
  seats: SessionSeat[];
}) {
  const { t } = useTranslation();
  const available = agents.filter((agent) => agent.availabilityState === "available");

  return (
    <section className="grid gap-2">
      <h4 className="text-xs font-semibold uppercase text-muted-foreground">{t("session.seats")}</h4>
      <ul className="grid gap-1.5">
        {seats.map((seat, index) => {
          const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
          const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
          return (
            <li className="ucd-list-row flex items-center gap-2 rounded-lg p-2 text-xs" key={index}>
              <span
                aria-hidden="true"
                className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border"
                style={{ borderColor: role?.color }}
              >
                {role?.avatar ?? "🤖"}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate font-medium">
                  {role?.displayName ?? agent?.displayName ?? seat.agentId}
                </span>
                <span className="block truncate text-muted-foreground">
                  {agent?.displayName ?? seat.agentId} · {agent?.modelFamily ?? "unknown"}
                </span>
              </span>
              <span className="shrink-0 text-muted-foreground">
                {activity[index] === "speaking" ? t("session.seatSpeaking") : t("session.seatIdle")}
              </span>
              <Button
                className="h-7 w-7 shrink-0 px-0"
                disabled={seats.length <= 1}
                onClick={() => {
                  const result = removeSeat(seats, index);
                  if (result) onSeatsChange(result.seats);
                }}
                title={t("session.seatRemove")}
                type="button"
                variant="outline"
              >
                <Trash2 aria-hidden="true" className="h-3 w-3" />
              </Button>
            </li>
          );
        })}
      </ul>
      <Button
        className="h-7 px-2 text-xs"
        disabled={available.length === 0}
        onClick={() => onSeatsChange(addSeat(seats, { agentId: available[0].id, roleId: null }).seats)}
        type="button"
        variant="outline"
      >
        <Plus aria-hidden="true" className="h-3 w-3" />
        {t("session.seatAdd")}
      </Button>
    </section>
  );
}
