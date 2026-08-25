import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import type { AgentService } from "../services/agent-service";
import type {
  WorkspaceCapabilityState,
  WorkspaceInspectionCapabilities,
} from "../types/session-workspace";

/**
 * Which capability a panel depends on.
 *
 * Named per panel rather than derived, because the mapping is a judgement: the Changes tab needs
 * both Git reads and would be useless with one, while Files needs listing and degrades to a list
 * with no previews without `readTextFiles`.
 */
export type WorkspaceCapabilityKey = keyof Omit<
  WorkspaceInspectionCapabilities,
  "provider" | "targetLabel" | "watchMode"
>;

export function useWorkspaceCapabilities(
  sessionId: string | null,
  service: AgentService = defaultAgentService,
): { capabilities: WorkspaceInspectionCapabilities | undefined; isLoading: boolean } {
  const query = useQuery({
    enabled: sessionId !== null,
    queryKey: ["workspace-inspection-capabilities", sessionId],
    queryFn: () => {
      if (sessionId === null) throw new Error("Workspace capabilities need a session.");
      return service.getWorkspaceInspectionCapabilities(sessionId);
    },
  });
  return { capabilities: query.data, isLoading: query.isLoading };
}

/**
 * Why a panel cannot show what it normally shows.
 *
 * Rendered instead of the panel's contents, and never instead of the workspace: the Shell needs
 * none of these prerequisites, so a missing helper or a missing ripgrep must not take away the one
 * thing that still works. That is the whole shape of this component — it replaces a region, not a
 * tab.
 *
 * A remediation is shown when there is one. "Search is unavailable" and "install ripgrep on the
 * remote host" are different facts, and only the second is something a reader can act on.
 */
export function WorkspaceCapabilityNotice({
  capability,
  targetLabel,
}: {
  capability: WorkspaceCapabilityState;
  /** Absent for a local workspace, which is what a reader assumes when nothing says otherwise. */
  targetLabel?: string;
}) {
  const { t } = useTranslation();
  if (capability.available) return null;

  const reason = capability.reasonCode ?? "workspace_provider_unavailable";
  return (
    <div
      className="ucd-status-warning flex flex-col gap-1 rounded-lg border p-3 text-sm"
      role="status"
    >
      <p>
        {t(`workspace.capability.reason.${reason}`, {
          defaultValue: t("workspace.capability.reason.workspace_provider_unavailable"),
        })}
      </p>
      {targetLabel ? (
        // Which machine, when it is not this one. Without it, "the workspace is unavailable" reads
        // as a fault in the application rather than a fact about a host.
        <p className="text-xs text-muted-foreground">
          {t("workspace.capability.target", { target: targetLabel })}
        </p>
      ) : null}
      {capability.remediation ? (
        <p className="text-xs text-muted-foreground">
          {t(`workspace.capability.remediation.${capability.remediation}`, {
            defaultValue: "",
          })}
        </p>
      ) : null}
      {/* Stated rather than implied. A reader looking at a panel that cannot answer needs to know
          the terminal is still there, because the alternative reading is that the session is
          unreachable. */}
      <p className="text-xs text-muted-foreground">{t("workspace.capability.shellStillAvailable")}</p>
    </div>
  );
}

/**
 * How stale a view can be, when it can be stale at all.
 *
 * Only rendered for polling and `none`, because those are the two where a reader has to do
 * something: press refresh, or know that an external change will never appear. `native` and
 * `event-derived` are shown as nothing, which is the correct amount of interface for "this keeps
 * itself up to date".
 */
export function WorkspaceWatchNotice({
  capabilities,
}: {
  capabilities: WorkspaceInspectionCapabilities | undefined;
}) {
  const { t } = useTranslation();
  if (!capabilities) return null;
  if (capabilities.watchMode === "native" || capabilities.watchMode === "event-derived") {
    return null;
  }
  return (
    <p className="text-xs text-muted-foreground" role="status">
      {t(`workspace.capability.watch.${capabilities.watchMode}`)}
    </p>
  );
}
