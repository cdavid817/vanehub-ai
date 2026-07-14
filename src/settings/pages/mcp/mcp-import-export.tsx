import { Clipboard, Download, Upload, X } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "../../../components/ui/button";
import type { McpImportExport, McpScope, McpServerConfig } from "../../../types/mcp";

export function McpImportExportModal({
  servers,
  onCancel,
  onImport,
  onExport,
}: {
  servers: McpServerConfig[];
  onCancel: () => void;
  onImport: (data: McpImportExport, scope: McpScope) => Promise<string>;
  onExport: (names: string[]) => Promise<McpImportExport>;
}) {
  const [mode, setMode] = useState<"import" | "export">("import");
  const [scope, setScope] = useState<McpScope>("user");
  const [input, setInput] = useState('{\n  "mcpServers": {}\n}');
  const [selected, setSelected] = useState<string[]>(servers.map((server) => server.name));
  const [output, setOutput] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const importNames = useMemo(() => {
    try {
      const parsed = JSON.parse(input) as McpImportExport;
      return Object.keys(parsed.mcpServers ?? {});
    } catch {
      return [];
    }
  }, [input]);

  async function handleImport() {
    setError(null);
    setMessage(null);
    try {
      const parsed = JSON.parse(input) as McpImportExport;
      setMessage(await onImport(parsed, scope));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleExport() {
    setError(null);
    setMessage(null);
    try {
      const data = await onExport(selected);
      setOutput(JSON.stringify(data, null, 2));
      setMessage(`已导出 ${Object.keys(data.mcpServers).length} 个服务器`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function copyOutput() {
    await navigator.clipboard.writeText(output);
    setMessage("已复制到剪贴板");
  }

  return (
    <div className="fixed inset-0 z-30 flex items-center justify-center bg-black/40 p-4">
      <section className="ucd-panel max-h-[90vh] w-full max-w-3xl overflow-y-auto rounded-lg p-4">
        <div className="mb-4 flex items-center justify-between gap-3">
          <div className="flex gap-2">
            <Button variant={mode === "import" ? "default" : "outline"} onClick={() => setMode("import")}>
              <Upload className="h-4 w-4" aria-hidden="true" />
              导入
            </Button>
            <Button variant={mode === "export" ? "default" : "outline"} onClick={() => setMode("export")}>
              <Download className="h-4 w-4" aria-hidden="true" />
              导出
            </Button>
          </div>
          <button className="rounded-md p-2 hover:bg-muted" onClick={onCancel} type="button" title="关闭">
            <X className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>

        {mode === "import" ? (
          <div className="grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">导入 scope</span>
              <select className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring" value={scope} onChange={(event) => setScope(event.target.value as McpScope)}>
                <option value="user">用户配置</option>
                <option value="project">项目配置</option>
              </select>
            </label>
            <textarea className="ucd-input min-h-72 rounded p-3 font-mono text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring" value={input} onChange={(event) => setInput(event.target.value)} />
            {importNames.length ? <div className="text-xs text-muted-foreground">预览：{importNames.join(", ")}</div> : null}
            <Button onClick={() => void handleImport()}>
              <Upload className="h-4 w-4" aria-hidden="true" />
              确认导入
            </Button>
          </div>
        ) : (
          <div className="grid gap-3">
            <div className="grid gap-2 md:grid-cols-2">
              {servers.map((server) => (
                <label className="flex items-center gap-2 rounded border border-border p-2 text-sm" key={server.name}>
                  <input
                    checked={selected.includes(server.name)}
                    onChange={(event) =>
                      setSelected((current) =>
                        event.target.checked ? [...current, server.name] : current.filter((name) => name !== server.name),
                      )
                    }
                    type="checkbox"
                  />
                  <span className="truncate">{server.name}</span>
                </label>
              ))}
            </div>
            <Button onClick={() => void handleExport()}>
              <Download className="h-4 w-4" aria-hidden="true" />
              生成 JSON
            </Button>
            <textarea readOnly className="ucd-input min-h-64 rounded p-3 font-mono text-xs outline-none" value={output} />
            <Button variant="outline" onClick={() => void copyOutput()} disabled={!output}>
              <Clipboard className="h-4 w-4" aria-hidden="true" />
              复制
            </Button>
          </div>
        )}

        {message ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-success">{message}</div> : null}
        {error ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      </section>
    </div>
  );
}
