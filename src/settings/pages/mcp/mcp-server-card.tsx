import { Edit3, PlayCircle, Power, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { ActionMenu, type ActionMenuItem } from "../../../ui/actions/ActionMenu";
import { MutationStatus } from "../../../ui/async/MutationStatus";
import type { MutationState } from "../../../ui/async/mutation-state";
import { CopyDiagnosticsButton } from "../../../ui/diagnostics/CopyDiagnosticsButton";
import { StatusBadge } from "../../../ui/status/StatusBadge";
import type { McpServerConfig, McpServerStatus } from "../../../types/mcp";
import { buildMcpDiagnosticFields } from "./mcp-diagnostic-summary";
import { mcpConnectionStatusKey, mcpConnectionStatusTone, mcpTransportTranslationKey } from "./mcp-presentation";
import { McpTestResultPanel } from "./mcp-test-result";

export function McpServerCard({
  deleteState,
  onDelete,
  onEdit,
  onTest,
  onToggle,
  server,
  status,
  testState,
  toggleState,
}: {
  deleteState: MutationState | undefined;
  onDelete: (server: McpServerConfig) => void;
  onEdit: (server: McpServerConfig) => void;
  onTest: (server: McpServerConfig) => void;
  onToggle: (server: McpServerConfig) => void;
  server: McpServerConfig;
  status?: McpServerStatus;
  testState: MutationState | undefined;
  toggleState: MutationState | undefined;
}) {
  const { t } = useTranslation();
  const endpoint = server.transportType === "stdio" ? [server.command, ...(server.args ?? [])].filter(Boolean).join(" ") : server.url;

  // Task 12.18: the page previously rendered a standalone toggle button plus three more
  // (Test/Edit/Delete) -- collapsed into one ActionMenu per card, matching Extensions' own
  // enable/disable-inside-the-menu precedent for this same kind of active/inactive action.
  const items: ActionMenuItem[] = [
    {
      disabled: toggleState?.pending,
      icon: Power,
      id: "toggle",
      label: t(server.active ? "mcp.toggle.disableNamed" : "mcp.toggle.enableNamed", { name: server.name }),
      onSelect: () => onToggle(server),
    },
    {
      disabled: testState?.pending,
      icon: PlayCircle,
      id: "test",
      label: testState?.pending ? t("mcp.action.testing") : t("mcp.action.test"),
      onSelect: () => onTest(server),
    },
    {
      icon: Edit3,
      id: "edit",
      label: t("mcp.action.edit"),
      onSelect: () => onEdit(server),
    },
    {
      confirmation: { title: t("mcp.confirm.delete", { name: server.name }) },
      disabled: deleteState?.pending,
      icon: Trash2,
      id: "delete",
      label: t("mcp.action.delete"),
      onSelect: () => onDelete(server),
      tone: "destructive",
    },
  ];

  return (
    <article className="ucd-panel ucd-interactive grid gap-3 rounded-lg p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="truncate text-sm font-semibold">{server.name}</h3>
          <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">
            <Badge tone="muted">{t(mcpTransportTranslationKey(server.transportType))}</Badge>
            <Badge tone={server.scope === "project" ? "warning" : "muted"}>{t(`mcp.scope.${server.scope}`)}</Badge>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <StatusBadge label={t(mcpConnectionStatusKey(status?.connectionStatus))} tone={mcpConnectionStatusTone(status?.connectionStatus)} />
          <ActionMenu items={items} triggerLabel={t("mcp.rowActions", { name: server.name })} />
        </div>
      </div>

      {server.description ? <p className="text-xs text-muted-foreground">{server.description}</p> : null}
      <div className="min-h-8 rounded border border-border bg-muted p-2 text-[11px] text-muted-foreground">
        <span className="break-all">{endpoint || t("mcp.connection.unconfigured")}</span>
      </div>

      <McpTestResultPanel status={status} />
      <MutationStatus state={toggleState} />
      <MutationStatus state={testState} />
      <MutationStatus state={deleteState} />
      <div className="flex justify-end">
        <CopyDiagnosticsButton fields={buildMcpDiagnosticFields(server, status, t)} />
      </div>
    </article>
  );
}
