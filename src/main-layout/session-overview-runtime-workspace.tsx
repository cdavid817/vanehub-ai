import type { ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Activity, Bot, Brain, FolderGit2, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { resolveModelLabel } from "../components/chat/models";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { normalizeDisplayPath } from "../lib/session-path";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { InspectorProviderContext } from "../ui/inspector/inspector-provider-registry";
import { SessionEvidenceSummary } from "./session-evidence-summary";

/**
 * Mirrors session-info-panel.tsx's own private `Field` layout. Duplicated rather than imported:
 * that file is untouched until its own removal (task 9.17) and never exported this helper.
 */
function Field({ icon, label, value }: { icon: ReactNode; label: string; value: ReactNode }) {
  return (
    <div className="border-b border-border/60 pb-2 last:border-0 last:pb-0">
      <dt className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span className="truncate">{label}</span>
      </dt>
      <dd className="mt-1 min-h-5 wrap-break-word text-sm font-medium">{value}</dd>
    </div>
  );
}

/**
 * CLI/agent identity, lifecycle state, and model — one half of the old Basic Info tab
 * (session-info-panel.tsx lines ~142-145), split from the Workspace half below because the two
 * describe different things: who/what is running this session, versus where its files live.
 */
export function SessionOverviewRuntimeSection({ session }: { session: Session }) {
  const { t } = useTranslation();
  const identity = getAgentVisualIdentity(session.agentId);
  // Ported as-is from session-info-panel.tsx: that query lived in the panel's own top-level scope,
  // not inside a per-tab pane, so it was never gated on which tab was active. Left ungated here
  // too, rather than newly tying it to this section's open/closed state.
  const chatConfig = useQuery({
    queryKey: ["session-chat-config", session.id],
    queryFn: () => agentService.getSessionChatConfig(session.id),
  });
  const modelLabel = resolveModelLabel(chatConfig.data?.providerId, chatConfig.data?.modelId) || null;

  return (
    <dl className="grid gap-2">
      <Field icon={<Bot aria-hidden="true" className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.session")} value={session.title} />
      <Field
        icon={<Sparkles aria-hidden="true" className="h-3.5 w-3.5 text-primary" />}
        label={t("layout.info.cli")}
        value={
          <span className="flex min-w-0 items-center gap-2">
            <span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", identity.tone)}>
              <AgentBrandIcon agentId={session.agentId} className="h-3.5 w-3.5" />
            </span>
            <span className="truncate">{identity.label}</span>
          </span>
        }
      />
      <Field icon={<Activity aria-hidden="true" className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.lifecycle")} value={t(`layout.lifecycle.${session.lifecycleState}`)} />
      <Field icon={<Brain aria-hidden="true" className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.model")} value={modelLabel ?? t("layout.info.modelUnavailable")} />
    </dl>
  );
}

export interface SessionOverviewWorkspaceSectionProps {
  /** Whether this section is the one currently expanded — gates SessionEvidenceSummary's own read. */
  active: boolean;
  context: InspectorProviderContext;
  /** Already resolved by the caller: `worktreePath ?? projectPath ?? folder`. */
  displayPath: string | null;
  onShowUsage: () => void;
  session: Session;
}

/**
 * Where this session's files live, plus the same seven-line evidence summary the old panel showed
 * beneath the path fields — the other half of the old Basic Info tab.
 */
export function SessionOverviewWorkspaceSection({ active, context, displayPath, onShowUsage, session }: SessionOverviewWorkspaceSectionProps) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-3">
      <dl className="grid gap-2">
        <Field
          icon={<FolderGit2 aria-hidden="true" className="h-3.5 w-3.5 text-primary" />}
          label={t("layout.info.workspace")}
          value={displayPath ? normalizeDisplayPath(displayPath) : t("layout.info.workspaceUnavailable")}
        />
      </dl>
      <SessionEvidenceSummary
        active={active}
        onNavigateToTab={context.onNavigateToSessionTab}
        onShowUsage={onShowUsage}
        sessionId={session.id}
      />
    </div>
  );
}
