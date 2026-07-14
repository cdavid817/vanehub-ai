import { Edit3, PlayCircle, Power, Trash2 } from "lucide-react";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { McpServerConfig, McpServerStatus } from "../../../types/mcp";
import { McpTestResultPanel } from "./mcp-test-result";

const transportLabels = {
  stdio: "stdio",
  sse: "sse",
  streamable_http: "http",
};

function statusLabel(status?: McpServerStatus) {
  if (!status) return "未测试";
  if (status.connectionStatus === "disabled") return "已禁用";
  if (status.connectionStatus === "connected") return "测试通过";
  if (status.connectionStatus === "error") return "测试失败";
  return "未测试";
}

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
  const endpoint = server.transportType === "stdio" ? [server.command, ...(server.args ?? [])].filter(Boolean).join(" ") : server.url;
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
            <Badge tone={server.scope === "project" ? "warning" : "muted"}>{server.scope === "project" ? "项目配置" : "用户配置"}</Badge>
            <span>{statusLabel(status)}</span>
          </div>
        </div>
        <button
          className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border hover:bg-muted"
          onClick={() => onToggle(server)}
          title={server.active ? "禁用" : "启用"}
          type="button"
        >
          <Power className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>

      {server.description ? <p className="mb-3 text-xs text-muted-foreground">{server.description}</p> : null}
      <div className="mb-3 min-h-8 rounded border border-border bg-muted p-2 text-[11px] text-muted-foreground">
        <span className="break-all">{endpoint || "未配置连接参数"}</span>
      </div>

      <McpTestResultPanel status={status} />

      <div className="mt-3 flex flex-wrap justify-end gap-2">
        <Button variant="outline" onClick={() => onTest(server)} disabled={testing}>
          <PlayCircle className="h-4 w-4" aria-hidden="true" />
          {testing ? "测试中" : "测试"}
        </Button>
        <Button variant="outline" onClick={() => onEdit(server)}>
          <Edit3 className="h-4 w-4" aria-hidden="true" />
          编辑
        </Button>
        <Button variant="ghost" onClick={() => onDelete(server)}>
          <Trash2 className="h-4 w-4" aria-hidden="true" />
          删除
        </Button>
      </div>
    </section>
  );
}
