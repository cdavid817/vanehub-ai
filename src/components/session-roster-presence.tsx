import { cn } from "../lib/utils";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "./agent-brand-icon";
import { Code2, DraftingCompass, ScanSearch, type LucideIcon } from "lucide-react";
import { activeSeatsFromSession } from "../services/session-seats";
import { seatDisplayName } from "../services/seat-presentation";
import type { Session, SessionSeat } from "../types/agent";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";

export type ParticipantRoleKind = "architect" | "implementer" | "reviewer";

const roleVisuals: Record<ParticipantRoleKind, { Icon: LucideIcon; iconClassName: string }> = {
  architect: { Icon: DraftingCompass, iconClassName: "text-violet-500" },
  implementer: { Icon: Code2, iconClassName: "text-emerald-500" },
  reviewer: { Icon: ScanSearch, iconClassName: "text-amber-500" },
};

export function participantRoleKind(roleId?: string | null, roleName?: string | null): ParticipantRoleKind | null {
  const identity = `${roleId ?? ""} ${roleName ?? ""}`.toLocaleLowerCase();
  if (identity.includes("builtin-architect") || identity.includes("architect") || identity.includes("架构")) return "architect";
  if (identity.includes("builtin-implementer") || identity.includes("implementer") || identity.includes("实现")) return "implementer";
  if (identity.includes("builtin-reviewer") || /(?:^|[\s_-])(?:reviewer|review)(?:$|[\s_-])/.test(identity) || identity.includes("审查") || identity.includes("评审")) return "reviewer";
  return null;
}

export function ParticipantAvatar({
  agentId,
  current = false,
  label,
  roleAvatar,
  roleId,
  roleName,
  size = "md",
  status = false,
}: {
  agentId: string;
  current?: boolean;
  label: string;
  roleAvatar?: string | null;
  roleId?: string | null;
  roleName?: string | null;
  size?: "sm" | "md";
  status?: boolean;
}) {
  const roleKind = participantRoleKind(roleId, roleName);
  const roleVisual = roleKind ? roleVisuals[roleKind] : null;
  const RoleIcon = roleVisual?.Icon;
  const roleIconName = roleKind ?? (roleName && roleAvatar ? "custom" : "agent");
  return (
    <span
      aria-hidden="true"
      className={cn(
        "relative flex shrink-0 items-center justify-center rounded-lg border bg-[hsl(var(--panel))] text-muted-foreground shadow-xs",
        size === "sm" ? "h-7 w-7" : "h-9 w-9",
        current && "border-primary bg-[hsl(var(--nav-active-soft))] text-primary ring-2 ring-primary/20",
      )}
      data-role-icon={roleIconName}
      title={label}
    >
      {roleVisual && RoleIcon ? (
        <RoleIcon
          aria-hidden="true"
          className={cn(size === "sm" ? "h-3.5 w-3.5" : "h-4 w-4", current ? "text-primary" : roleVisual.iconClassName)}
        />
      ) : roleName && roleAvatar ? (
        <span className={size === "sm" ? "text-xs" : "text-sm"}>{roleAvatar}</span>
      ) : (
        <AgentBrandIcon agentId={agentId} className={size === "sm" ? "h-3.5 w-3.5" : "h-4 w-4"} title="" />
      )}
      {status ? (
        <span
          className={cn(
            "absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-background",
            current ? "bg-[hsl(var(--success))]" : "bg-muted-foreground",
          )}
        />
      ) : null}
    </span>
  );
}

function ParticipantMarker({ seat }: { seat: SessionSeat }) {
  const label = `${seatDisplayName(seat)} · ${seat.roleSnapshot?.agentName ?? seat.agentId}`;
  return <ParticipantAvatar agentId={seat.agentId} label={label} roleAvatar={seat.roleSnapshot?.avatar} roleId={seat.roleId} roleName={seat.roleSnapshot?.roleName} size="sm" status />;
}

export function SessionRosterAvatars({ session }: { session: Session }) {
  const { t } = useTranslation();
  const seats = activeSeatsFromSession(session);
  if (seats.length < 2) return null;
  const visible = seats.slice(0, 3);
  return (
    <span aria-label={t("session.participantCount", { count: seats.length })} className="flex shrink-0 -space-x-1.5">
      {visible.map((seat, index) => <ParticipantMarker key={seat.seatId ?? `${seat.agentId}:${index}`} seat={seat} />)}
      {seats.length > visible.length ? (
        <span className="flex h-7 w-7 items-center justify-center rounded-lg border-2 border-background bg-muted text-[10px] font-semibold shadow-xs">
          +{seats.length - visible.length}
        </span>
      ) : null}
    </span>
  );
}

export function SessionRosterChips({
  currentSeatId,
  session,
  showSingle = false,
}: {
  currentSeatId?: string | null;
  session: Session;
  showSingle?: boolean;
}) {
  const { t } = useTranslation();
  const seats = activeSeatsFromSession(session);
  if (!seats.length || (seats.length < 2 && !showSingle)) return null;
  return (
    <div
      className="mt-2 flex max-w-full gap-2 overflow-x-auto overflow-y-hidden pb-0.5"
      data-layout="single-row"
      data-testid="session-roster-chips"
      role="list"
    >
      {seats.map((seat, index) => {
        const current = Boolean(currentSeatId && currentSeatId === seat.seatId);
        const agentName = seat.agentId;
        const resolvedName = seatDisplayName(seat);
        const name = resolvedName === agentName ? getAgentVisualIdentity(agentName).label : resolvedName;
        return (
          <span
            aria-current={current ? "true" : undefined}
            className={cn(
              "grid h-12 w-56 shrink-0 grid-cols-[1.75rem_minmax(0,1fr)_3.25rem] items-center gap-2 rounded-md border border-border/80 bg-background/60 py-1.5 pl-1.5 pr-2 text-xs text-muted-foreground shadow-xs",
              current && "border-primary bg-[hsl(var(--nav-active-soft))] text-foreground",
            )}
            data-seat-id={seat.seatId}
            data-speaking={current ? "true" : "false"}
            key={seat.seatId ?? `${seat.agentId}:${index}`}
            role="listitem"
            title={`${name} · ${agentName}`}
          >
            <ParticipantAvatar agentId={seat.agentId} current={current} label={`${name} · ${agentName}`} roleAvatar={seat.roleSnapshot?.avatar} roleId={seat.roleId} roleName={seat.roleSnapshot?.roleName} size="sm" status />
            <span className="min-w-0 flex-1">
              <span className="block truncate font-medium text-foreground">{name}</span>
              <span className="block truncate text-[10px]">{agentName}</span>
            </span>
            <span
              aria-hidden={current ? undefined : "true"}
              className={cn(
                "justify-self-end whitespace-nowrap rounded-full bg-[hsl(var(--nav-active-soft))] px-1.5 py-0.5 text-[10px] font-medium text-primary",
                !current && "invisible",
              )}
              data-testid="participant-speaking-state"
            >
              {t("session.seatSpeaking")}
            </span>
          </span>
        );
      })}
    </div>
  );
}
