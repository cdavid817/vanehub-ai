import { Edit3, PlayCircle, Power, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { McpServerConfig, McpServerStatus } from "../../../types/mcp";
import { McpTestResultPanel } from "./mcp-test-result";

const transportLabels = {
  stdio: "stdio",
  sse: "sse",
  streamable_http: "http",
};

export function McpServerCard({
  server,
  status,
  testing,
  onEdit,
  onDelete,
  onTest,
  onToggle,
}: {
  server: McpServerConfig;
  status?: McpServerStatus;
  testing: boolean;
  onEdit: (server: McpServerConfig) => void;
  onDelete: (server: McpServerConfig) => void;
  onTest: (server: McpServerConfig) => void;
  onToggle: (server: McpServerConfig) => void;
}) {
  const { t } = useTranslation();
  const endpoint = server.transportType === "stdio" ? [server.command, ...(server.args ?? [])].filter(Boolean).join(" ") : server.url;
  const statusKey = status?.connectionStatus === "disabled"
    ? "mcp.status.disabled"
    : status?.connectionStatus === "connected"
      ? "mcp.status.connected"
      : status?.connectionStatus === "error"
        ? "mcp.status.error"
        : "mcp.status.notTested";
  return (
    <section className="ucd-panel rounded-lg p-4">
      <div className="mb-3 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className={`h-2.5 w-2.5 rounded-full ${server.active ? "bg-[#22c55e]" : "bg-[#94a3b8]"}`} />
            <h3 className="truncate text-sm font-semibold">{server.name}</h3>
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">
            <Badge tone="muted">{transportLabels[server.transportType]}</Badge>
            <Badge tone={server.scope === "project" ? "warning" : "muted"}>{t(`mcp.scope.${server.scope}`)}</Badge>
            <span>{t(statusKey)}</span>
          </div>
        </div>
        <button
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border hover:bg-muted"
          onClick={() => onToggle(server)}
          title={server.active ? t("mcp.toggle.disable") : t("mcp.toggle.enable")}
          type="button"
        >
          <Power className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>

      {server.description ? <p className="mb-3 text-xs text-muted-foreground">{server.description}</p> : null}
      <div className="mb-3 min-h-8 rounded border border-border bg-muted p-2 text-[11px] text-muted-foreground">
        <span className="break-all">{endpoint || t("mcp.connection.unconfigured")}</span>
      </div>

      <McpTestResultPanel status={status} />

      <div className="mt-3 flex flex-wrap justify-end gap-2">
        <Button variant="outline" onClick={() => onTest(server)} disabled={testing}>
          <PlayCircle className="h-4 w-4" aria-hidden="true" />
          {testing ? t("mcp.action.testing") : t("mcp.action.test")}
        </Button>
        <Button variant="outline" onClick={() => onEdit(server)}>
          <Edit3 className="h-4 w-4" aria-hidden="true" />
          {t("mcp.action.edit")}
        </Button>
        <Button variant="ghost" onClick={() => onDelete(server)}>
          <Trash2 className="h-4 w-4" aria-hidden="true" />
          {t("mcp.action.delete")}
        </Button>
      </div>
    </section>
  );
}
