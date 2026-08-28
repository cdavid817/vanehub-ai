import { useTranslation } from "react-i18next";
import type { AgentPersonalizationCapability } from "../../../types/personalization";
import type { MemoryQuery } from "../../../types/personalization-memory";
import type { WorkspaceOption } from "./use-scope-options";

const STATUSES = ["active", "archived"] as const;
const TYPES = ["user", "feedback", "project", "reference"] as const;
const SCOPES = ["any", "global", "workspace"] as const;

function Select({
  children,
  label,
  onChange,
  testId,
  value,
}: {
  children: React.ReactNode;
  label: string;
  onChange: (value: string) => void;
  testId: string;
  value: string;
}) {
  return (
    <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
      {label}
      <select
        className="ucd-input h-9 rounded-md px-2 text-sm"
        data-testid={testId}
        onChange={(event) => onChange(event.target.value)}
        value={value}
      >
        {children}
      </select>
    </label>
  );
}

/**
 * Six filters over the paged list.
 *
 * Source Agent and Agent audience are separate controls because they answer different questions --
 * who recorded a memory, and who may read it. One Agent recording something another one reads is
 * ordinary, and a single control would make that memory unfindable from one of the two sides.
 *
 * A filter change is reported without a cursor. The parent starts the result set over, because a
 * cursor names a position in one filtered ordering and means nothing in another.
 */
export function MemoryFilters({
  agents,
  onChange,
  query,
  workspaces,
}: {
  agents: readonly AgentPersonalizationCapability[];
  onChange: (patch: Omit<MemoryQuery, "cursor">) => void;
  query: MemoryQuery;
  workspaces: readonly WorkspaceOption[];
}) {
  const { t } = useTranslation();

  return (
    <div
      className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3"
      data-testid="personalization-memory-filters"
    >
      <label className="flex min-w-0 flex-col gap-1 text-xs font-medium">
        {t("personalization.memoryList.filters.search")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid="personalization-memory-search"
          onChange={(event) => onChange({ text: event.target.value || undefined })}
          placeholder={t("personalization.memoryList.filters.searchPlaceholder")}
          type="search"
          value={query.text ?? ""}
        />
      </label>

      <Select
        label={t("personalization.memoryList.filters.scope")}
        onChange={(value) =>
          onChange({
            scopeKind: value === "any" ? undefined : (value as "global" | "workspace"),
            // A workspace filter without a workspace addresses nothing, so the first workspace is
            // selected with it rather than leaving a control that reads as "all workspaces".
            workspaceKey: value === "workspace" ? (query.workspaceKey ?? workspaces[0]?.workspaceKey) : undefined,
          })
        }
        testId="personalization-memory-scope"
        value={query.scopeKind ?? "any"}
      >
        {SCOPES.map((scope) => (
          <option key={scope} value={scope}>
            {t(`personalization.memoryList.scope.${scope}`)}
          </option>
        ))}
      </Select>

      {query.scopeKind === "workspace" ? (
        <Select
          label={t("personalization.scope.workspace")}
          onChange={(value) => onChange({ workspaceKey: value || undefined })}
          testId="personalization-memory-workspace"
          value={query.workspaceKey ?? ""}
        >
          <option value="">{t("personalization.scope.chooseWorkspace")}</option>
          {workspaces.map((workspace) => (
            <option key={workspace.workspaceKey} value={workspace.workspaceKey}>
              {workspace.displayName}
            </option>
          ))}
        </Select>
      ) : null}

      <Select
        label={t("personalization.memoryList.filters.status")}
        onChange={(value) => onChange({ status: value ? (value as "active" | "archived") : undefined })}
        testId="personalization-memory-status"
        value={query.status ?? ""}
      >
        <option value="">{t("personalization.memoryList.anyStatus")}</option>
        {STATUSES.map((status) => (
          <option key={status} value={status}>
            {t(`personalization.memoryList.status.${status}`)}
          </option>
        ))}
      </Select>

      <Select
        label={t("personalization.memoryList.filters.type")}
        onChange={(value) => onChange({ memoryType: value ? (value as (typeof TYPES)[number]) : undefined })}
        testId="personalization-memory-type"
        value={query.memoryType ?? ""}
      >
        <option value="">{t("personalization.memoryList.anyType")}</option>
        {TYPES.map((type) => (
          <option key={type} value={type}>
            {t(`personalization.memory.type.${type}`)}
          </option>
        ))}
      </Select>

      <Select
        label={t("personalization.memoryList.filters.sourceAgent")}
        onChange={(value) => onChange({ sourceAgentId: value || undefined })}
        testId="personalization-memory-source-agent"
        value={query.sourceAgentId ?? ""}
      >
        <option value="">{t("personalization.memoryList.anyAgent")}</option>
        {agents.map((agent) => (
          <option key={agent.agentId} value={agent.agentId}>
            {agent.displayName}
          </option>
        ))}
      </Select>

      <Select
        label={t("personalization.memoryList.filters.audienceAgent")}
        onChange={(value) => onChange({ audienceAgentId: value || undefined })}
        testId="personalization-memory-audience-agent"
        value={query.audienceAgentId ?? ""}
      >
        <option value="">{t("personalization.memoryList.anyAgent")}</option>
        {agents.map((agent) => (
          <option key={agent.agentId} value={agent.agentId}>
            {agent.displayName}
          </option>
        ))}
      </Select>
    </div>
  );
}
