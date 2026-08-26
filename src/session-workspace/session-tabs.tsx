import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { LazyFeature } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type { Session, SessionSeat } from "../types/agent";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import { useSessionRoles } from "../hooks/use-session-speakers";
import { activeSeatsFromSession, seatsFromSession } from "../services/session-seats";
import { SeatSwitcher } from "./seat-switcher";
import { effectiveSeatId, showsSeatSwitcher, tabScope } from "./tab-scope";
import type { ChatMessage } from "../types/chat";
import { AgentTerminalTab } from "./agent-terminal-tab";
import { ChatTab } from "./chat-tab";
import { SessionTabBar, sessionTabDefinitions, type SessionTabId } from "./session-tab-bar";
import { ConversationOverflowMenu } from "./conversation-overflow-menu";
import { SessionConversationHeader } from "./session-conversation-header";
import { useWorkspaceInvalidation } from "./use-workspace-invalidation";
import { useMountedWorkspaceTabs } from "./use-mounted-workspace-tabs";
import {
  useWorkspaceEvidenceNotices,
  useWorkspaceEvidenceSummary,
} from "./use-workspace-evidence-summary";
import { workspaceTabBadges, type WorkspaceTabBadge } from "./workspace-evidence-badges";
import { evidenceTabOf } from "./workspace-evidence-reducer";
import {
  WorkspaceEvidenceScopeProvider,
  useWorkspaceEvidenceScope,
} from "./workspace-evidence-scope";
import { WorkspaceEvidenceScopeChips } from "./workspace-evidence-scope-chips";

export interface ConversationVisibilityControls {
  infoPanelExpanded: boolean;
  onToggleInfoPanel: () => void;
  onOpenIm: () => void;
  onToggleSessionList: () => void;
  onToggleWorkspaceTabs: () => void;
  sessionListExpanded: boolean;
  workspaceTabsExpanded: boolean;
}

interface SessionTabsProps {
  activeSession: Session | null;
  apiComposer?: ReactNode;
  focusMode?: boolean;
  isStreaming?: boolean;
  messages: ChatMessage[];
  messagesPartial: boolean;
  onLoadEarlier?: () => void;
  onOpenSettings: () => void;
  recoveryNotice?: ReactNode;
  requestedTab?: SessionTabId | null;
  requestedTabNonce?: number;
  sessionActivationKey: number;
  /** Null in a single-seat session, which has no turn to hand off. */
  turnStatus?: TurnStatus | null;
  visibilityControls?: ConversationVisibilityControls;
  workspaceTabsCollapsed?: boolean;
}

const loadChangesTab = () => import("./changes-tab").then((module) => ({ default: module.ChangesTab }));
const loadDocumentsTab = () => import("./documents-tab").then((module) => ({ default: module.DocumentsTab }));
const loadFilesTab = () => import("./files-tab").then((module) => ({ default: module.FilesTab }));
const loadTerminalTab = () => import("./terminal-tab").then((module) => ({ default: module.TerminalTab }));
const loadShellTab = () => import("./shell-tab").then((module) => ({ default: module.ShellTab }));
const loadLogsTab = () => import("./logs-tab").then((module) => ({ default: module.LogsTab }));
const loadExecutionTimelineTab = () => import("./execution-timeline-tab")
  .then((module) => ({ default: module.ExecutionTimelineTab }));
const loadReportTab = () => import("./report-tab").then((module) => ({ default: module.ReportTab }));

/**
 * Owns the evidence scope for the selected session and nothing else.
 *
 * The split exists because a component cannot read a context it provides, and the active tab now
 * lives in that context: it moves together with the scope, and a navigation that changed one
 * without the other would show a panel filtered to a row the user never chose.
 */
export function SessionTabs(props: SessionTabsProps) {
  const seats = useMemo(
    () => (props.activeSession ? activeSeatsFromSession(props.activeSession) : []),
    [props.activeSession],
  );
  const seatIds = useMemo(
    () => seats.flatMap((seat) => (seat.seatId === undefined ? [] : [seat.seatId])),
    [seats],
  );
  // Parsed rather than asserted: the brand is a claim that the value passed validation, and the
  // schema is the only place allowed to make it. A session without a usable id gets no scope,
  // which is the honest answer — not an empty scope that reads as "no filters applied".
  const sessionId = useMemo(() => {
    const parsed = evidenceSessionIdSchema.safeParse(props.activeSession?.id);
    return parsed.success ? parsed.data : null;
  }, [props.activeSession?.id]);

  // One summary read and one notice subscription for the whole workspace, above the panels so
  // neither multiplies by the number of mounted tabs.
  const { state, summary } = useWorkspaceEvidenceSummary(sessionId);
  const { recordsRevision } = useWorkspaceEvidenceNotices(sessionId);
  const badges = useMemo(() => workspaceTabBadges(summary, state), [state, summary]);

  return (
    <WorkspaceEvidenceScopeProvider seatIds={seatIds} sessionId={sessionId}>
      <SessionWorkspaceTabs
        {...props}
        badges={badges}
        recordsRevision={recordsRevision}
        seats={seats}
      />
    </WorkspaceEvidenceScopeProvider>
  );
}

function SessionWorkspaceTabs({
  activeSession,
  apiComposer,
  badges,
  focusMode = false,
  isStreaming = false,
  messages,
  messagesPartial,
  onLoadEarlier = () => undefined,
  onOpenSettings,
  recordsRevision,
  recoveryNotice,
  requestedTab,
  requestedTabNonce = 0,
  seats,
  sessionActivationKey,
  turnStatus = null,
  visibilityControls,
  workspaceTabsCollapsed = false,
}: SessionTabsProps & {
  badges: Partial<Record<SessionTabId, WorkspaceTabBadge>>;
  /** Bumped when a live notice said records moved, so a paged panel can re-read its newest page. */
  recordsRevision: number;
  seats: SessionSeat[];
}) {
  const { t } = useTranslation();
  const sessionId = activeSession?.id ?? null;
  const isSharedThread = Boolean(activeSession && seatsFromSession(activeSession).length > 1);
  const { activateTab, activeTab } = useWorkspaceEvidenceScope();
  // Null is "all seats": a freshly opened tab must not silently narrow to one participant.
  const [selectedSeat, setSelectedSeat] = useState<number | null>(null);
  const roles = useSessionRoles(seats.length > 1);
  const { mount, mountedTabs } = useMountedWorkspaceTabs(sessionId, activeTab);
  // Mounted here rather than in each panel: the panels come and go as a reader switches tabs, and a
  // subscription that came and went with them would leave a window on every switch in which a
  // notice is published to nobody. Nothing on screen would say one had been missed.
  useWorkspaceInvalidation(sessionId);

  useEffect(() => {
    setSelectedSeat(null);
  }, [sessionId]);

  useEffect(() => {
    if (!requestedTab) return;
    mount(requestedTab);
    activateTab(requestedTab);
    // The nonce lets the same tab be requested twice in a row — otherwise a second `/logs` after
    // the user manually returned to chat would be a no-op. The session reset happens during
    // render, so an uncleared request still wins over the switch back to chat.
  }, [activateTab, mount, requestedTab, requestedTabNonce, sessionId]);

  function activate(tab: SessionTabId) {
    mount(tab);
    activateTab(tab);
  }

  function renderPanel(id: SessionTabId) {
    // Resolved per tab rather than read from the switcher: a session-scoped tab must not change
    // because a control that does not apply to it happens to hold a selection.
    const seatId = effectiveSeatId(id, seats, selectedSeat);
    // Every panel is told whether it is on screen. A mounted panel that cannot tell keeps polling,
    // refreshing, and re-aggregating for a view nobody is reading — and the retention policy in
    // the capability registry is a claim nothing enforces.
    const isVisible = activeTab === id;
    if (id === "chat") {
      if (activeSession?.interactionMode === "api" || isSharedThread) {
        return (
          <ChatTab
            activeSession={activeSession}
            composer={apiComposer}
            messages={messages}
            onLoadEarlier={onLoadEarlier}
            turnStatus={turnStatus}
          />
        );
      }
      return <AgentTerminalTab isVisible={activeTab === "chat"} session={activeSession} sessionActivationKey={sessionActivationKey} />;
    }
    if (id === "changes") return <LazyFeature componentProps={{ isVisible, sessionId }} loader={loadChangesTab} />;
    if (id === "documents") return <LazyFeature componentProps={{ isVisible, sessionId }} loader={loadDocumentsTab} />;
    if (id === "files") return <LazyFeature componentProps={{ isVisible, onNavigateToShell: () => activateTab("shell"), sessionId }} loader={loadFilesTab} />;
    if (id === "terminal") {
      return <LazyFeature componentProps={{ builtinToolsAvailable: activeSession?.agentId === "onepiece", isVisible, messages, partial: messagesPartial, recordsRevision, seatId, sessionId, targetRoot: activeSession?.worktreePath ?? activeSession?.projectPath ?? "" }} loader={loadTerminalTab} />;
    }
    if (id === "shell") {
      return <LazyFeature componentProps={{ isVisible, seatId, sessionId }} loader={loadShellTab} />;
    }
    if (id === "logs") return <LazyFeature componentProps={{ isVisible, seatId, sessionId }} loader={loadLogsTab} />;
    if (id === "traces") return <LazyFeature componentProps={{ isVisible, session: activeSession, sessionId }} loader={loadExecutionTimelineTab} />;
    return <LazyFeature componentProps={{ isVisible, sessionId }} loader={loadReportTab} />;
  }

  return (
    <div
      className="flex h-full min-h-0 flex-col"
      data-focus-mode={focusMode ? "true" : "false"}
      data-testid="session-workspace"
    >
      <SessionConversationHeader
        actions={visibilityControls ? <ConversationOverflowMenu {...visibilityControls} /> : null}
        isStreaming={isStreaming}
        onOpenIm={visibilityControls?.onOpenIm}
        session={activeSession}
      />
      {recoveryNotice}
      {focusMode || workspaceTabsCollapsed ? null : (
        <div className="shrink-0 border-b border-border/70 bg-[hsl(var(--panel))] px-3 py-2">
          <SessionTabBar
            activeTab={activeTab}
            badges={badges}
            onActivate={activate}
            onOpenSettings={onOpenSettings}
            session={activeSession}
          />
        </div>
      )}
      <div className="min-h-0 flex-1 overflow-hidden">
        {sessionTabDefinitions.map(({ id }) => mountedTabs.includes(id) ? (
          <section
            aria-labelledby={`session-tab-${id}`}
            className={cn("h-full min-h-0", activeTab === id ? "block" : "hidden")}
            // Which session this panel is showing, for verification that has to reach a native
            // registry keyed by session id. Nothing on screen carries it, and a desktop test that
            // guessed would be checking a different session's state than the one it is looking at.
            data-session-id={sessionId ?? ""}
            id={`session-tab-panel-${id}`}
            key={`${sessionId ?? "none"}-${id}`}
            role="tabpanel"
          >
            {showsSeatSwitcher(id, seats.length) ? (
              <SeatSwitcher
                onSelect={setSelectedSeat}
                roles={roles}
                seats={seats}
                selectedIndex={selectedSeat}
              />
            ) : null}
            {seats.length > 1 && tabScope(id) === "session" ? (
              // Without this the absent switcher reads as an omission rather than as a statement
              // that the tab is not seat-scoped.
              <p className="sr-only">{t("session.seatSwitcher.sessionScoped")}</p>
            ) : null}
            <ScopeChipsFor tab={id} />
            {renderPanel(id)}
          </section>
        ) : null)}
      </div>
    </div>
  );
}

/** Chat consumes no evidence scope, so it gets no chips rather than an empty chip bar. */
function ScopeChipsFor({ tab }: { tab: SessionTabId }) {
  const destination = evidenceTabOf(tab);
  return destination === null ? null : <WorkspaceEvidenceScopeChips tab={destination} />;
}
