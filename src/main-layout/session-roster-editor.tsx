import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { History, Plus, Users } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { agentService } from "../services/runtime-agent-client";
import { withModelFamily } from "../services/agent-model-family";
import { snapshotSeat, seatDisplayName } from "../services/seat-presentation";
import { activeSeatsFromSession, seatsFromSession } from "../services/session-seats";
import type { Session, SessionSeat } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { isSessionAgentSelectable, selectSessionAgents } from "./create-session-agents";
import { SessionSeatsPanel } from "./session-seats-panel";

export function SessionRosterEditor({
  currentSpeakerSeatId = null,
  messages = [],
  session,
}: {
  currentSpeakerSeatId?: string | null;
  messages?: ChatMessage[];
  session: Session;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<SessionSeat[]>(() => activeSeatsFromSession(session));
  const [agentId, setAgentId] = useState("");
  const [roleId, setRoleId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const agents = useQuery({ queryKey: ["agents"], queryFn: () => agentService.listAgents() });
  const roles = useQuery({ queryKey: ["expert-roles"], queryFn: () => agentService.listExpertRoles() });
  const rosterAgents = useMemo(() => withModelFamily(agents.data ?? []), [agents.data]);
  const availableAgents = useMemo(
    () => selectSessionAgents(rosterAgents).filter(isSessionAgentSelectable),
    [rosterAgents],
  );
  const departed = seatsFromSession(session).filter((seat) => seat.leftAt != null);

  useEffect(() => {
    setDraft(activeSeatsFromSession(session));
    setError(null);
  }, [session]);
  useEffect(() => {
    if (!agentId && availableAgents[0]) setAgentId(availableAgents[0].id);
  }, [agentId, availableAgents]);

  const update = useMutation({
    mutationFn: (seats: SessionSeat[]) => agentService.updateSessionSeats({
      sessionId: session.id,
      expectedUpdatedAt: session.updatedAt,
      seats: seats.map((seat) => snapshotSeat(seat, agents.data ?? [], roles.data ?? [])),
    }),
    onSuccess: (updated) => {
      setDraft(activeSeatsFromSession(updated));
      queryClient.setQueryData(["sessions", "active"], updated);
      void queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
    onError: async () => {
      setError(t("session.membershipConflict"));
      const current = await agentService.getSession(session.id).catch(() => null);
      if (current) {
        setDraft(activeSeatsFromSession(current));
        queryClient.setQueryData(["sessions", "active"], current);
      }
    },
  });

  function save(next: SessionSeat[]) {
    setError(null);
    setDraft(next);
    update.mutate(next);
  }

  return (
    <section aria-label={t("session.memberInfo")} className="ucd-muted-panel grid gap-2 rounded-lg border-primary/20 p-3" data-testid="session-roster-editor">
      <div className="flex items-center justify-between gap-2">
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <Users aria-hidden="true" className="h-4 w-4 text-primary" />
          {t("session.memberInfo")}
        </h3>
        <span className="rounded-full bg-[hsl(var(--nav-active-soft))] px-2 py-0.5 text-xs font-semibold tabular-nums text-primary">
          {draft.length}
        </span>
      </div>
      <SessionSeatsPanel
        agents={rosterAgents}
        allowQuickAdd={false}
        disabled={update.isPending}
        onSeatsChange={save}
        roles={roles.data ?? []}
        seats={draft}
        messages={messages}
        speakingSeatId={currentSpeakerSeatId}
      />
      <div className="grid grid-cols-2 gap-2 border-t border-border/60 pt-3">
        <select aria-label={t("createSession.seatAgent")} className="ucd-input h-9 min-w-0 rounded px-2 text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setAgentId(event.target.value)} value={agentId}>
          {availableAgents.map((agent) => <option key={agent.id} value={agent.id}>{agent.displayName}</option>)}
        </select>
        <select aria-label={t("createSession.seatRole")} className="ucd-input h-9 min-w-0 rounded px-2 text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setRoleId(event.target.value)} value={roleId}>
          <option value="">{t("createSession.seatRoleNone")}</option>
          {(roles.data ?? []).map((role) => <option key={role.id} value={role.id}>{role.displayName}</option>)}
        </select>
        <Button
          className="col-span-2 h-9 px-3 text-xs"
          disabled={!agentId || update.isPending}
          onClick={() => save([...draft, { agentId, roleId: roleId || null }])}
          title={t("session.seatAdd")}
          type="button"
          variant="outline"
        >
          <Plus aria-hidden="true" className="h-3.5 w-3.5" />
          {t("session.seatAdd")}
        </Button>
      </div>
      {departed.length ? (
        <details className="rounded-lg border border-border/60 bg-background/50 p-2 text-xs text-muted-foreground">
          <summary className="flex cursor-pointer items-center gap-2 rounded px-1 py-0.5 hover:text-foreground">
            <History aria-hidden="true" className="h-3.5 w-3.5" />
            {t("session.departedParticipants", { count: departed.length })}
          </summary>
          <ul className="mt-2 grid gap-1.5 border-t border-border/60 pt-2">
            {departed.map((seat, index) => <li className="flex items-center justify-between gap-2 px-1" key={seat.seatId ?? index}><span className="truncate">{seatDisplayName(seat)}</span><span>{t("session.departed")}</span></li>)}
          </ul>
        </details>
      ) : null}
      {error ? <p className="text-xs text-destructive" role="alert">{error}</p> : null}
    </section>
  );
}
