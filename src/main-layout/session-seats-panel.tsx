import { Circle, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentWithModelFamily } from "../services/agent-model-family";
import { addSeat, removeSeat } from "../services/seat-mutation";
import type { ExpertRole } from "../types/expert-role";
import type { SessionSeat } from "../types/agent";
import { ParticipantAvatar } from "../components/session-roster-presence";

/**
 * Shows who is in the session and lets the line-up change while it runs, because the collaboration
 * path emerges during the work.
 *
 * Deliberately absent: any control that picks who speaks next. Routing belongs to the Agents and to
 * `@` mentions; offering a dispatch button here would quietly turn the human into a supervisor.
 */
export function SessionSeatsPanel({
  agents,
  allowQuickAdd = true,
  disabled = false,
  onSeatsChange,
  roles,
  seats,
  speakingSeatId = null,
}: {
  agents: AgentWithModelFamily[];
  allowQuickAdd?: boolean;
  disabled?: boolean;
  onSeatsChange: (seats: SessionSeat[]) => void;
  roles: ExpertRole[];
  seats: SessionSeat[];
  speakingSeatId?: string | null;
}) {
  const { t } = useTranslation();
  const available = agents.filter((agent) => agent.availabilityState === "available");

  return (
    <section className="grid gap-2">
      <h4 className="sr-only">{t("session.seats")}</h4>
      <ul className="grid gap-2">
        {seats.map((seat, index) => {
          const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
          const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
          const speaking = Boolean(seat.seatId && seat.seatId === speakingSeatId);
          return (
            <li
              aria-current={speaking ? "true" : undefined}
              className="ucd-list-row flex items-center gap-2 rounded-lg p-2.5 text-xs"
              data-speaking={speaking ? "true" : "false"}
              key={seat.seatId ?? `${seat.agentId}:${index}`}
            >
              <ParticipantAvatar
                agentId={seat.agentId}
                current={speaking}
                label={`${seat.roleSnapshot?.roleName ?? role?.displayName ?? agent?.displayName ?? seat.agentId} · ${seat.roleSnapshot?.agentName ?? agent?.displayName ?? seat.agentId}`}
                roleAvatar={seat.roleSnapshot?.avatar ?? role?.avatar}
                roleId={seat.roleId}
                roleName={seat.roleSnapshot?.roleName ?? role?.displayName}
                status
              />
              <span className="min-w-0 flex-1">
                <span className="block truncate font-medium">
                  {seat.roleSnapshot?.roleName ?? role?.displayName ?? agent?.displayName ?? seat.agentId}
                </span>
                <span className="block truncate text-muted-foreground">
                  {seat.roleSnapshot?.agentName ?? agent?.displayName ?? seat.agentId} · {seat.roleSnapshot?.modelFamily ?? agent?.modelFamily ?? "unknown"}
                </span>
              </span>
              <span className="inline-flex shrink-0 items-center gap-1 rounded-full bg-muted px-2 py-1 text-[10px] text-muted-foreground">
                <Circle aria-hidden="true" className="h-1.5 w-1.5 fill-current" />
                {speaking ? t("session.seatSpeaking") : t("session.seatIdle")}
              </span>
              <Button
                aria-label={t("session.seatLeave")}
                className="h-9 w-9 shrink-0 px-0 text-muted-foreground hover:text-destructive"
                disabled={disabled || seats.length <= 1}
                onClick={() => {
                  const result = removeSeat(seats, index);
                  if (result) onSeatsChange(result.seats);
                }}
                title={t("session.seatLeave")}
                type="button"
                variant="outline"
              >
                <Trash2 aria-hidden="true" className="h-4 w-4" />
              </Button>
            </li>
          );
        })}
      </ul>
      {allowQuickAdd ? <Button
        className="h-9 px-3 text-xs"
        disabled={disabled || available.length === 0}
        onClick={() => onSeatsChange(addSeat(seats, { agentId: available[0].id, roleId: null }).seats)}
        type="button"
        variant="outline"
      >
        <Plus aria-hidden="true" className="h-3.5 w-3.5" />
        {t("session.seatAdd")}
      </Button> : null}
    </section>
  );
}
