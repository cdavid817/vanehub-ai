import { useTranslation } from "react-i18next";
import type {
  AgentPersonalizationCapability,
  PersonalizationPolicyRef,
  PolicyScopeKind,
} from "../../../types/personalization";
import type { WorkspaceOption } from "./use-scope-options";

const SCOPE_KINDS: PolicyScopeKind[] = ["global", "agent", "workspace", "workspace-agent"];

/**
 * Picks which layer is being edited.
 *
 * The Agent choices come from the registry, never from a fixed set of checkboxes: a fixed set is
 * wrong the day an Agent is added, and wrong in a way nobody notices, because the missing Agent
 * simply has no control rather than an obviously broken one.
 *
 * A scope named after a key is not selectable until that key is chosen. Reporting the incomplete
 * selection here keeps the page from asking the native side to address a layer that does not exist.
 */
export function PersonalizationScopeSelector({
  agents,
  onChange,
  scope,
  workspaces,
}: {
  agents: readonly AgentPersonalizationCapability[];
  onChange: (scope: PersonalizationPolicyRef) => void;
  scope: PersonalizationPolicyRef;
  workspaces: readonly WorkspaceOption[];
}) {
  const { t } = useTranslation();
  const needsAgent = scope.scopeKind === "agent" || scope.scopeKind === "workspace-agent";
  const needsWorkspace = scope.scopeKind === "workspace" || scope.scopeKind === "workspace-agent";

  return (
    <div className="flex flex-col gap-3 sm:flex-row sm:items-end" data-testid="personalization-scope-selector">
      <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
        {t("personalization.scope.layer")}
        <select
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-scope-kind"
          onChange={(event) => onChange(nextScope(event.target.value as PolicyScopeKind, scope))}
          value={scope.scopeKind}
        >
          {SCOPE_KINDS.map((kind) => (
            <option key={kind} value={kind}>
              {t(`personalization.scope.kind.${kind}`)}
            </option>
          ))}
        </select>
      </label>

      {needsAgent ? (
        <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.agent")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-scope-agent"
            onChange={(event) => onChange({ ...scope, agentId: event.target.value || undefined })}
            value={scope.agentId ?? ""}
          >
            <option value="">{t("personalization.scope.chooseAgent")}</option>
            {agents.map((agent) => (
              <option key={agent.agentId} value={agent.agentId}>
                {agent.displayName}
              </option>
            ))}
          </select>
        </label>
      ) : null}

      {needsWorkspace ? (
        <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.workspace")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid="personalization-scope-workspace"
            onChange={(event) => onChange({ ...scope, workspaceKey: event.target.value || undefined })}
            value={scope.workspaceKey ?? ""}
          >
            <option value="">{t("personalization.scope.chooseWorkspace")}</option>
            {workspaces.map((workspace) => (
              <option key={workspace.workspaceKey} value={workspace.workspaceKey}>
                {workspace.displayName}
              </option>
            ))}
          </select>
        </label>
      ) : null}

      {isIncomplete(scope) ? (
        <p className="text-xs text-muted-foreground sm:pb-2" data-testid="personalization-scope-incomplete" role="status">
          {t("personalization.scope.incomplete")}
        </p>
      ) : null}
    </div>
  );
}

/**
 * Keeps the keys a new layer still needs and drops the ones it does not.
 *
 * Carrying a stale Agent id into a workspace-only scope would send the native side a selection the
 * user cannot see, and the layer it addressed would not be the one on screen.
 */
export function nextScope(
  kind: PolicyScopeKind,
  current: PersonalizationPolicyRef,
): PersonalizationPolicyRef {
  return {
    scopeKind: kind,
    agentId: kind === "agent" || kind === "workspace-agent" ? current.agentId : undefined,
    workspaceKey:
      kind === "workspace" || kind === "workspace-agent" ? current.workspaceKey : undefined,
  };
}

export function isIncomplete(scope: PersonalizationPolicyRef): boolean {
  if (scope.scopeKind === "global") return false;
  if (scope.scopeKind === "agent") return !scope.agentId;
  if (scope.scopeKind === "workspace") return !scope.workspaceKey;
  return !scope.agentId || !scope.workspaceKey;
}
