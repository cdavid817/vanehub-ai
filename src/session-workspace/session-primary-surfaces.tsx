import { useEffect } from "react";
import { PanelBottomOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { LazyFeature } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import { seatsFromSession } from "../services/session-seats";
import type { EvidenceSessionId } from "../types/session-workspace-evidence";
import { AgentTerminalTab } from "./agent-terminal-tab";
import { ChatTab } from "./chat-tab";
import { ConversationOverflowMenu } from "./conversation-overflow-menu";
import { SessionConversationHeader } from "./session-conversation-header";
import { isPrimarySurface, type SessionPrimarySurfaceId } from "./session-surface-registry";
import { SessionTabBar, sessionTabDefinitions } from "./session-tab-bar";
import { useMountedWorkspaceTabs } from "./use-mounted-workspace-tabs";
import { useWorkspacePathNavigation } from "./use-workspace-path-navigation";
import { evidenceTabOf } from "./workspace-evidence-reducer";
import type { WorkspaceEvidenceNavigation } from "./workspace-evidence-scope";
import { WorkspaceEvidenceScopeChips } from "./workspace-evidence-scope-chips";
import { workspaceTabBadges } from "./workspace-evidence-badges";
import type { SessionWorkspaceRegionsProps } from "./session-tabs";

const loadChangesTab = () => import("./changes-tab").then((module) => ({ default: module.ChangesTab }));
const loadReportTab = () => import("./report-tab").then((module) => ({ default: module.ReportTab }));
const loadFilesSurface = () => import("./session-files-surface").then((module) => ({ default: module.SessionFilesSurface }));

export function SessionPrimarySurfaces(
  props: SessionWorkspaceRegionsProps & {
    badges: ReturnType<typeof workspaceTabBadges>;
    scope: WorkspaceEvidenceNavigation;
    sessionId: EvidenceSessionId | null;
  },
) {
  const {
    activeSession,
    apiComposer,
    badges,
    currentSelectionKey = null,
    focusMode = false,
    isStreaming = false,
    messages,
    onLoadEarlier = () => undefined,
    onOpenSettings,
    onSelectMessage,
    onSelectTool,
    onStop,
    recoveryNotice,
    requestedSurface,
    requestedSurfaceNonce = 0,
    scope,
    sessionActivationKey,
    sessionId,
    turnStatus = null,
    visibilityControls,
    workspaceTabsCollapsed = false,
  } = props;
  const { t } = useTranslation();
  const isSharedThread = Boolean(activeSession && seatsFromSession(activeSession).length > 1);
  const pathNavigation = useWorkspacePathNavigation();
  const { mount, mountedTabs } = useMountedWorkspaceTabs(sessionId, scope.activePrimarySurface);

  useEffect(() => {
    if (!requestedSurface) return;
    if (isPrimarySurface(requestedSurface)) mount(requestedSurface);
    scope.activateSurface(requestedSurface);
    // The nonce lets the same surface be requested twice in a row — otherwise a second `/logs`
    // after the user manually returned to Work would be a no-op. The session reset happens during
    // render, so an uncleared request still wins over the switch back to Work.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mount, requestedSurface, requestedSurfaceNonce, sessionId]);

  function activate(id: SessionPrimarySurfaceId) {
    mount(id);
    scope.activateSurface(id);
  }

  function renderPanel(id: SessionPrimarySurfaceId) {
    const isVisible = scope.activePrimarySurface === id;
    if (id === "work") {
      if (activeSession?.interactionMode === "api" || isSharedThread) {
        return (
          <ChatTab
            activeSession={activeSession}
            composer={apiComposer}
            currentSelectionKey={currentSelectionKey}
            messages={messages}
            onLoadEarlier={onLoadEarlier}
            onSelectMessage={onSelectMessage}
            onSelectTool={onSelectTool}
            turnStatus={turnStatus}
          />
        );
      }
      return <AgentTerminalTab isVisible={isVisible} session={activeSession} sessionActivationKey={sessionActivationKey} />;
    }
    if (id === "changes") return <LazyFeature componentProps={{ isVisible, onShowOperation: pathNavigation.showOperation, sessionId }} loader={loadChangesTab} />;
    if (id === "files") {
      return (
        <LazyFeature
          componentProps={{
            isVisible,
            onNavigateToShell: pathNavigation.showShell,
            onOpenChanges: pathNavigation.showChanges,
            onShowEvidence: pathNavigation.showEvidence,
            sessionId,
          }}
          loader={loadFilesSurface}
        />
      );
    }
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
        onStop={onStop}
        session={activeSession}
      />
      {recoveryNotice}
      {focusMode || workspaceTabsCollapsed ? null : (
        <div className="flex shrink-0 items-center gap-2 border-b border-border/70 bg-[hsl(var(--panel))] px-3 py-2">
          <SessionTabBar
            activeTab={scope.activePrimarySurface}
            badges={badges}
            onActivate={activate}
            onOpenSettings={onOpenSettings}
            session={activeSession}
          />
          {scope.runtimePanelOpen ? null : (
            // The tab strip that used to carry Terminal History/Shell/Logs/Traces is gone — this
            // is the one manual entry point into the Runtime Panel for a reader who has not been
            // sent there by a slash command, badge, or evidence link (design.md Decision 7's "Open
            // runtime evidence" scenario names "panel tab" as one of its own listed openers).
            <button
              aria-label={t("layout.runtimePanel")}
              className="ucd-interactive flex h-8 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
              onClick={() => scope.openRuntimePanel()}
              title={t("layout.runtimePanel")}
              type="button"
            >
              <PanelBottomOpen aria-hidden="true" className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      )}
      <div className="min-h-0 flex-1 overflow-hidden">
        {sessionTabDefinitions.map(({ id }) => mountedTabs.includes(id) ? (
          <section
            aria-labelledby={`session-tab-${id}`}
            className={cn("h-full min-h-0", scope.activePrimarySurface === id ? "block" : "hidden")}
            // Which session this panel is showing, for verification that has to reach a native
            // registry keyed by session id. Nothing on screen carries it, and a desktop test that
            // guessed would be checking a different session's state than the one it is looking at.
            data-session-id={sessionId ?? ""}
            id={`session-tab-panel-${id}`}
            key={`${sessionId ?? "none"}-${id}`}
            role="tabpanel"
          >
            <ScopeChipsFor tab={id} />
            {renderPanel(id)}
          </section>
        ) : null)}
      </div>
    </div>
  );
}

/** Work consumes no evidence scope, so it gets no chips rather than an empty chip bar. */
function ScopeChipsFor({ tab }: { tab: SessionPrimarySurfaceId }) {
  const destination = evidenceTabOf(tab);
  return destination === null ? null : <WorkspaceEvidenceScopeChips tab={destination} />;
}
