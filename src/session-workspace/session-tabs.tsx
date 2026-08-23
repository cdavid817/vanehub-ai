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
import { toolUseCount } from "./terminal-utils";
import { ConversationOverflowMenu } from "./conversation-overflow-menu";
import { SessionConversationHeader } from "./session-conversation-header";
import { useMountedWorkspaceTabs } from "./use-mounted-workspace-tabs";
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

  return (
    <WorkspaceEvidenceScopeProvider seatIds={seatIds} sessionId={sessionId}>
      <SessionWorkspaceTabs {...props} seats={seats} />
    </WorkspaceEvidenceScopeProvider>
  );
}

function SessionWorkspaceTabs({
  activeSession,
  apiComposer,
  focusMode = false,
  isStreaming = false,
  messages,
  messagesPartial,
  onLoadEarlier = () => undefined,
  onOpenSettings,
  recoveryNotice,
  requestedTab,
  requestedTabNonce = 0,
  seats,
  sessionActivationKey,
  turnStatus = null,
  visibilityControls,
  workspaceTabsCollapsed = false,
}: SessionTabsProps & { seats: SessionSeat[] }) {
  const { t } = useTranslation();
  const sessionId = activeSession?.id ?? null;
  const isSharedThread = Boolean(activeSession && seatsFromSession(activeSession).length > 1);
  const { activateTab, activeTab } = useWorkspaceEvidenceScope();
  // Null is "all seats": a freshly opened tab must not silently narrow to one participant.
  const [selectedSeat, setSelectedSeat] = useState<number | null>(null);
  const roles = useSessionRoles(seats.length > 1);
  const { mount, mountedTabs } = useMountedWorkspaceTabs(sessionId);
  const terminalCount = useMemo(() => toolUseCount(messages), [messages]);

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
      return <AgentTerminalTab active={activeTab === "chat"} session={activeSession} sessionActivationKey={sessionActivationKey} />;
    }
    if (id === "changes") return <LazyFeature componentProps={{ sessionId }} loader={loadChangesTab} />;
    if (id === "documents") return <LazyFeature componentProps={{ sessionId }} loader={loadDocumentsTab} />;
    if (id === "files") return <LazyFeature componentProps={{ sessionId }} loader={loadFilesTab} />;
    if (id === "terminal") {
      return <LazyFeature componentProps={{ builtinToolsAvailable: activeSession?.agentId === "onepiece", messages, partial: messagesPartial, seatId, sessionId, targetRoot: activeSession?.worktreePath ?? activeSession?.projectPath ?? "" }} loader={loadTerminalTab} />;
    }
    if (id === "shell") {
      return <LazyFeature componentProps={{ active: activeTab === "shell", seatId, sessionId }} loader={loadShellTab} />;
    }
    if (id === "logs") return <LazyFeature componentProps={{ seatId, sessionId }} loader={loadLogsTab} />;
    if (id === "traces") return <LazyFeature componentProps={{ session: activeSession, sessionId }} loader={loadExecutionTimelineTab} />;
    return <LazyFeature componentProps={{ messages, partial: messagesPartial }} loader={loadReportTab} />;
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
            badges={{ terminal: terminalCount > 0 ? terminalCount : undefined }}
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
