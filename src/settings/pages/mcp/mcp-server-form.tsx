import { Save, X } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "../../../components/ui/button";
import type { McpScope, McpServerConfig, McpTransportType } from "../../../types/mcp";

function jsonText(value: Record<string, string> | null | undefined) {
  return JSON.stringify(value ?? {}, null, 2);
}

function parseRecord(value: string, label: string): Record<string, string> {
  const parsed: unknown = value.trim() ? JSON.parse(value) : {};
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error(`${label} 必须是 JSON object`);
  }
  return Object.fromEntries(Object.entries(parsed as Record<string, unknown>).map(([key, item]) => [key, String(item)]));
}

function parseArgs(value: string) {
  const trimmed = value.trim();
  if (!trimmed) return [];
  if (trimmed.startsWith("[")) {
    const parsed: unknown = JSON.parse(trimmed);
    if (!Array.isArray(parsed)) throw new Error("args JSON 必须是数组");
    return parsed.map(String);
  }
  return trimmed.split(/\r?\n/).map((item) => item.trim()).filter(Boolean);
}

export function McpServerForm({
  server,
  onCancel,
  onSave,
}: {
  server?: McpServerConfig | null;
  onCancel: () => void;
  onSave: (server: McpServerConfig) => Promise<void>;
}) {
  const editingName = server?.name ?? null;
  const [name, setName] = useState(server?.name ?? "");
  const [transportType, setTransportType] = useState<McpTransportType>(server?.transportType ?? "stdio");
  const [scope, setScope] = useState<McpScope>(server?.scope ?? "user");
  const [command, setCommand] = useState(server?.command ?? "");
  const [args, setArgs] = useState((server?.args ?? []).join("\n"));
  const [env, setEnv] = useState(jsonText(server?.env));
  const [url, setUrl] = useState(server?.url ?? "");
  const [headers, setHeaders] = useState(jsonText(server?.headers));
  const [description, setDescription] = useState(server?.description ?? "");
  const [active, setActive] = useState(server?.active ?? true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const title = useMemo(() => (editingName ? "编辑 MCP 服务器" : "添加 MCP 服务器"), [editingName]);

  async function handleSubmit() {
    setError(null);
    setSaving(true);
    try {
      const next: McpServerConfig = {
        name: name.trim(),
        transportType,
        command: transportType === "stdio" ? command.trim() : null,
        args: transportType === "stdio" ? parseArgs(args) : null,
        env: transportType === "stdio" ? parseRecord(env, "env") : null,
        url: transportType !== "stdio" ? url.trim() : null,
        headers: transportType !== "stdio" ? parseRecord(headers, "headers") : null,
        description: description.trim() || null,
        active,
        scope,
      };
      await onSave(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-30 flex items-center justify-center bg-black/40 p-4">
      <section className="ucd-panel max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-lg p-4">
        <div className="mb-4 flex items-center justify-between gap-3">
          <h3 className="text-sm font-semibold">{title}</h3>
          <button className="rounded-md p-2 hover:bg-muted" onClick={onCancel} type="button" title="关闭">
            <X className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>

        <div className="grid gap-3 text-sm md:grid-cols-2">
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">名称</span>
            <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={name} onChange={(event) => setName(event.target.value)} />
          </label>
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">Scope</span>
            <select className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={scope} onChange={(event) => setScope(event.target.value as McpScope)}>
              <option value="user">用户配置</option>
              <option value="project">项目配置</option>
            </select>
          </label>
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">Transport</span>
            <select className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={transportType} onChange={(event) => setTransportType(event.target.value as McpTransportType)}>
              <option value="stdio">stdio</option>
              <option value="sse">sse</option>
              <option value="streamable_http">streamable_http</option>
            </select>
          </label>
          <label className="flex items-center gap-2 pt-5 text-sm">
            <input checked={active} onChange={(event) => setActive(event.target.checked)} type="checkbox" />
            启用
          </label>
        </div>

        <label className="mt-3 grid gap-1 text-sm">
          <span className="text-xs text-muted-foreground">描述</span>
          <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={description} onChange={(event) => setDescription(event.target.value)} />
        </label>

        {transportType === "stdio" ? (
          <div className="mt-3 grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Command</span>
              <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={command} onChange={(event) => setCommand(event.target.value)} />
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Args</span>
              <textarea className="ucd-input min-h-24 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={args} onChange={(event) => setArgs(event.target.value)} />
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Env JSON</span>
              <textarea className="ucd-input min-h-28 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={env} onChange={(event) => setEnv(event.target.value)} />
            </label>
          </div>
        ) : (
          <div className="mt-3 grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">URL</span>
              <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={url} onChange={(event) => setUrl(event.target.value)} />
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Headers JSON</span>
              <textarea className="ucd-input min-h-28 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={headers} onChange={(event) => setHeaders(event.target.value)} />
            </label>
          </div>
        )}

        {error ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}

        <div className="mt-4 flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel}>取消</Button>
          <Button onClick={() => void handleSubmit()} disabled={saving}>
            <Save className="h-4 w-4" aria-hidden="true" />
            {saving ? "保存中" : "保存"}
          </Button>
        </div>
      </section>
    </div>
  );
}
