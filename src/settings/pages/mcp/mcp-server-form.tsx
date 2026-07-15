import { Save, X } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "../../../components/ui/button";
import type { McpScope, McpServerConfig, McpTransportType } from "../../../types/mcp";
import { type McpServerFormErrors, validateMcpServerForm } from "./mcp-server-validation";

function jsonText(value: Record<string, string> | null | undefined) {
  return JSON.stringify(value ?? {}, null, 2);
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
  const [fieldErrors, setFieldErrors] = useState<McpServerFormErrors>({});
  const [saving, setSaving] = useState(false);

  const title = useMemo(() => (editingName ? "编辑 MCP 服务器" : "添加 MCP 服务器"), [editingName]);

  async function handleSubmit() {
    setError(null);
    setFieldErrors({});
    const result = validateMcpServerForm({
      name,
      transportType,
      scope,
      command,
      args,
      env,
      url,
      headers,
      description,
      active,
    });

    if (!result.success) {
      setFieldErrors(result.errors);
      setError(result.errors.form ?? "请修正表单中的错误");
      return;
    }

    setSaving(true);
    try {
      await onSave(result.config);
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
            {fieldErrors.name ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.name}</span> : null}
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
              {fieldErrors.command ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.command}</span> : null}
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Args</span>
              <textarea className="ucd-input min-h-24 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={args} onChange={(event) => setArgs(event.target.value)} />
              {fieldErrors.args ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.args}</span> : null}
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Env JSON</span>
              <textarea className="ucd-input min-h-28 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={env} onChange={(event) => setEnv(event.target.value)} />
              {fieldErrors.env ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.env}</span> : null}
            </label>
          </div>
        ) : (
          <div className="mt-3 grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">URL</span>
              <input className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={url} onChange={(event) => setUrl(event.target.value)} />
              {fieldErrors.url ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.url}</span> : null}
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">Headers JSON</span>
              <textarea className="ucd-input min-h-28 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={headers} onChange={(event) => setHeaders(event.target.value)} />
              {fieldErrors.headers ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.headers}</span> : null}
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
