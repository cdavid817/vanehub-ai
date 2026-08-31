import { type ReactNode } from "react";
import { Activity, ScrollText, Shell, TerminalSquare, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { LazyFeature } from "../components/lazy-feature";
import { RuntimePanel, type RuntimePanelTab } from "../ui/runtime-panel/RuntimePanel";
import type { Session, SessionSeat } from "../types/agent";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import type { ChatMessage } from "../types/chat";
import type { ExpertRole } from "../types/expert-role";
import { effectiveSeatId, showsSeatSwitcher, tabScope } from "./tab-scope";
import { SeatSwitcher } from "./seat-switcher";
import type { SessionRuntimeSurfaceId } from "./session-surface-registry";
import { evidenceTabOf } from "./workspace-evidence-reducer";
import { useWorkspaceEvidenceScope } from "./workspace-evidence-scope";
import { WorkspaceEvidenceScopeChips } from "./workspace-evidence-scope-chips";
import type { WorkspaceTabBadge } from "./workspace-evidence-badges";

const loadTerminalTab = () => import("./terminal-tab").then((module) => ({ default: module.TerminalTab }));
const loadShellTab = () => import("./shell-tab").then((module) => ({ default: module.ShellTab }));
const loadLogsTab = () => import("./logs-tab").then((module) => ({ default: module.LogsTab }));
const loadExecutionTimelineTab = () => import("./execution-timeline-tab")
  .then((module) => ({ default: module.ExecutionTimelineTab }));

const RUNTIME_TAB_ICONS: Record<SessionRuntimeSurfaceId, LucideIcon> = {
  "terminal-history": TerminalSquare,
  shell: Shell,
  logs: ScrollText,
  traces: Activity,
};

export interface SessionRuntimePanelContentProps {
  activeSession: Session | null;
  badges: Partial<Record<SessionRuntimeSurfaceId, WorkspaceTabBadge>>;
  maximized: boolean;
  messages: ChatMessage[];
  messagesPartial: boolean;
  onMaximizedChange: (maximized: boolean) => void;
  onSelectSeat: (index: number | null) => void;
  recordsRevision: number;
  roles: ExpertRole[];
  seats: SessionSeat[];
  selectedSeat: number | null;
  sessionId: string | null;
  turnStatus: TurnStatus | null;
}

/**
 * Wraps a runtime surface's own content with the seat switcher and scope chips every seat-scoped
 * surface already carried in the nine-tab model — the surface itself is unmodified, only its
 * container moved from a tabpanel section to a Runtime Panel tab.
 */
function RuntimeSurfaceShell({
  children,
  id,
  onSelectSeat,
  roles,
  seats,
  selectedSeat,
}: {
  children: ReactNode;
  id: SessionRuntimeSurfaceId;
  onSelectSeat: (index: number | null) => void;
  roles: ExpertRole[];
  seats: SessionSeat[];
  selectedSeat: number | null;
}) {
  const { t } = useTranslation();
  const destination = evidenceTabOf(id);
  return (
    <div className="flex h-full min-h-0 flex-col">
      {showsSeatSwitcher(id, seats.length) ? (
        <SeatSwitcher onSelect={onSelectSeat} roles={roles} seats={seats} selectedIndex={selectedSeat} />
      ) : null}
      {seats.length > 1 && tabScope(id) === "session" ? (
        // Without this the absent switcher reads as an omission rather than as a statement that
        // the surface is not seat-scoped.
        <p className="sr-only">{t("session.seatSwitcher.sessionScoped")}</p>
      ) : null}
      {destination === null ? null : <WorkspaceEvidenceScopeChips tab={destination} />}
      <div className="min-h-0 flex-1">{children}</div>
    </div>
  );
}

/** design.md 8.13: a required seat has no "everyone" reading, so Shell must not attach to one. */
function ShellSeatGate({ onSelectSeat, roles, seats }: { onSelectSeat: (index: number) => void; roles: ExpertRole[]; seats: SessionSeat[] }) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full min-h-0 flex-col items-center justify-center gap-3 p-6 text-center" data-testid="shell-seat-gate">
      <p className="text-sm text-muted-foreground">{t("session.shellSeatGate.prompt")}</p>
      <div className="flex flex-wrap justify-center gap-1.5">
        {seats.map((seat, index) => {
          const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
          return (
            <button
              className="ucd-interactive flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-xs"
              key={seat.seatId ?? index}
              onClick={() => onSelectSeat(index)}
              type="button"
            >
              <span aria-hidden="true">{role?.avatar ?? "🤖"}</span>
              <span className="truncate">{role?.displayName ?? seat.agentId}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

export function useSessionRuntimeTabs(props: SessionRuntimePanelContentProps): RuntimePanelTab[] {
  const { t } = useTranslation();
  const {
    activeSession,
    badges,
    messages,
    messagesPartial,
    onSelectSeat,
    recordsRevision,
    roles,
    seats,
    selectedSeat,
    sessionId,
  } = props;

  function runtimeTab(id: SessionRuntimeSurfaceId, content: ReactNode): RuntimePanelTab {
    const badge = badges[id];
    return {
      badge: badge === undefined || badge.kind === "none" ? undefined : <TabBadgeGlyph badge={badge} />,
      icon: RUNTIME_TAB_ICONS[id],
      id,
      label: t(`sessionTabs.tab.${id}`),
      render: () => (
        <RuntimeSurfaceShell id={id} onSelectSeat={onSelectSeat} roles={roles} seats={seats} selectedSeat={selectedSeat}>
          {content}
        </RuntimeSurfaceShell>
      ),
    };
  }

  const shellSeatId = effectiveSeatId("shell", seats, selectedSeat);
  const shellNeedsSeat = seats.length > 1 && shellSeatId === null;

  return [
    runtimeTab(
      "terminal-history",
      <LazyFeature
        componentProps={{
          builtinToolsAvailable: activeSession?.agentId === "onepiece",
          isVisible: true,
          messages,
          partial: messagesPartial,
          recordsRevision,
          seatId: effectiveSeatId("terminal-history", seats, selectedSeat),
          sessionId,
          targetRoot: activeSession?.worktreePath ?? activeSession?.projectPath ?? "",
        }}
        loader={loadTerminalTab}
      />,
    ),
    runtimeTab(
      "shell",
      shellNeedsSeat ? (
        <ShellSeatGate onSelectSeat={onSelectSeat} roles={roles} seats={seats} />
      ) : (
        <LazyFeature componentProps={{ isVisible: true, seatId: shellSeatId, sessionId }} loader={loadShellTab} />
      ),
    ),
    runtimeTab(
      "logs",
      <LazyFeature
        componentProps={{ isVisible: true, seatId: effectiveSeatId("logs", seats, selectedSeat), sessionId }}
        loader={loadLogsTab}
      />,
    ),
    runtimeTab(
      "traces",
      <LazyFeature componentProps={{ isVisible: true, session: activeSession, sessionId }} loader={loadExecutionTimelineTab} />,
    ),
  ];
}

function TabBadgeGlyph({ badge }: { badge: WorkspaceTabBadge }) {
  if (badge.kind === "unknown") {
    return (
      <span aria-hidden="true" className="min-w-5 rounded-full border border-dashed border-border px-1 text-center font-mono text-[10px] text-muted-foreground">
        ·
      </span>
    );
  }
  if (badge.kind === "none") return null;
  return (
    <span
      aria-hidden="true"
      className={`min-w-5 rounded-full border px-1 font-mono text-[10px] ${badge.tone === "danger" ? "border-destructive text-destructive" : "border-border"}`}
    >
      {badge.atLeast ? `≥${badge.count}` : badge.count}
    </span>
  );
}

export function SessionRuntimePanel(props: SessionRuntimePanelContentProps & { className?: string }) {
  const { t } = useTranslation();
  const { activeRuntimeSurface, activateSurface, closeRuntimePanel } = useWorkspaceEvidenceScope();
  const tabs = useSessionRuntimeTabs(props);

  return (
    <RuntimePanel
      activeTabId={activeRuntimeSurface}
      ariaLabel={t("layout.runtimePanel")}
      className={props.className}
      maximized={props.maximized}
      onActiveTabChange={(id) => activateSurface(id as SessionRuntimeSurfaceId)}
      onClose={closeRuntimePanel}
      onMaximizedChange={props.onMaximizedChange}
      tabs={tabs}
    />
  );
}
