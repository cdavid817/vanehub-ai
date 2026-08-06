import { Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { AgentWithModelFamily } from "../services/agent-model-family";
import { recommendReviewerAgents } from "../services/reviewer-recommendation";
import { isSessionAgentSelectable } from "./create-session-dialog-utils";
import type { ExpertRole, SessionSeat } from "../types/agent-seats";

/**
 * Seats pair one expert role with one Agent for this session only, so the same installed CLI can
 * review here and architect elsewhere. There is deliberately no control for choosing who speaks
 * next: routing belongs to the Agents and to `@` mentions, not to a dispatcher.
 */
export function SessionSeatAssignment({
  agents,
  onSeatsChange,
  roles,
  seats,
}: {
  agents: AgentWithModelFamily[];
  onSeatsChange: (seats: SessionSeat[]) => void;
  roles: ExpertRole[];
  seats: SessionSeat[];
}) {
  const { t } = useTranslation();
  // The same rule the rest of this dialog uses. A stricter one here empties the seat editor while
  // the single-Agent selector stays full, leaving Create disabled with nothing saying why.
  const available = agents.filter(isSessionAgentSelectable);

  function update(index: number, patch: Partial<SessionSeat>) {
    onSeatsChange(seats.map((seat, position) => (position === index ? { ...seat, ...patch } : seat)));
  }

  return (
    <section className="grid gap-2">
      <span className="text-xs font-medium text-muted-foreground">{t("createSession.seats")}</span>
      {seats.map((seat, index) => {
        const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
        // A reviewer seat is judged against the seat above it — the work it would be reviewing.
        const reviewing = index > 0 ? seats[index - 1].agentId : null;
        const recommendation =
          role?.reviewPolicy.requireDifferentFamily && reviewing
            ? recommendReviewerAgents(available, reviewing)
            : null;
        const options = recommendation?.agents ?? available;

        return (
          <div className="ucd-list-row grid gap-2 rounded-lg p-2" key={index}>
            <div className="flex items-center gap-2">
              <select
                aria-label={t("createSession.seatRole")}
                className="ucd-input h-8 flex-1 rounded px-2 text-xs"
                onChange={(event) => update(index, { roleId: event.target.value || null })}
                value={seat.roleId ?? ""}
              >
                <option value="">{t("createSession.seatRoleNone")}</option>
                {roles.map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.avatar} {candidate.displayName}
                  </option>
                ))}
              </select>
              <Button
                className="h-8 w-8 px-0"
                disabled={seats.length <= 1}
                onClick={() => onSeatsChange(seats.filter((_, position) => position !== index))}
                title={t("createSession.seatRemove")}
                type="button"
                variant="outline"
              >
                <Trash2 aria-hidden="true" className="h-3.5 w-3.5" />
              </Button>
            </div>

            <select
              aria-label={t("createSession.seatAgent")}
              className="ucd-input h-8 rounded px-2 text-xs"
              onChange={(event) => update(index, { agentId: event.target.value })}
              value={seat.agentId}
            >
              {options.map((agent) => (
                <option key={agent.id} value={agent.id}>
                  {agent.displayName} · {agent.modelFamily}
                </option>
              ))}
            </select>

            {recommendation?.degraded ? (
              <span className="text-[11px] text-warning">
                {t("createSession.seatCrossFamilyUnavailable")}
              </span>
            ) : null}
            {role ? <span className="text-[11px] text-muted-foreground">{role.responsibility}</span> : null}
          </div>
        );
      })}

      <Button
        className="h-8 px-3 text-xs"
        onClick={() =>
          onSeatsChange([...seats, { agentId: available[0]?.id ?? "", roleId: null }])
        }
        type="button"
        variant="outline"
      >
        <Plus aria-hidden="true" className="h-3.5 w-3.5" />
        {t("createSession.seatAdd")}
      </Button>
    </section>
  );
}
