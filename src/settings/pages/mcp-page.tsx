import { Boxes, Plus, RefreshCw, Upload } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "../../components/ui/button";
import { mcpService } from "../../services/runtime-mcp-client";
import type { McpImportExport, McpScope, McpServerConfig, McpServerStatus, McpTestResult } from "../../types/mcp";
import { PageHeader, SectionPanel, StatCard } from "./page-parts";
import { McpImportExportModal } from "./mcp/mcp-import-export";
import { McpServerCard } from "./mcp/mcp-server-card";
import { McpServerForm } from "./mcp/mcp-server-form";

type StatusMap = Record<string, McpServerStatus>;

export function McpPage({ searchTerm }: { searchTerm: string }) {
  const [servers, setServers] = useState<McpServerConfig[]>([]);
  const [statuses, setStatuses] = useState<StatusMap>({});
  const [testingName, setTestingName] = useState<string | null>(null);
  const [editingServer, setEditingServer] = useState<McpServerConfig | null | undefined>();
  const [showImportExport, setShowImportExport] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setError(null);
    const nextServers = await mcpService.listServers();
    setServers(nextServers);
    const entries = await Promise.all(
      nextServers.map(async (server) => [server.name, await mcpService.getServerStatus(server.name)] as const),
    );
    setStatuses(Object.fromEntries(entries));
  }

  useEffect(() => {
    void refresh().catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, []);

  const visibleServers = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return servers;
    return servers.filter((server) =>
      [server.name, server.description ?? "", server.command ?? "", server.url ?? ""].some((value) =>
        value.toLowerCase().includes(query),
      ),
    );
  }, [searchTerm, servers]);

  const userServers = visibleServers.filter((server) => server.scope === "user");
  const projectServers = visibleServers.filter((server) => server.scope === "project");
  const totalTools = Object.values(statuses).reduce((sum, status) => sum + status.tools.length, 0);
  const connectedCount = Object.values(statuses).filter((status) => status.connectionStatus === "connected").length;
  const averageDuration = Math.round(
    Object.values(statuses).reduce((sum, status) => sum + (status.durationMs ?? 0), 0) /
      Math.max(1, Object.values(statuses).filter((status) => status.durationMs).length),
  );

  async function saveServer(server: McpServerConfig) {
    if (editingServer?.name) {
      await mcpService.updateServer(editingServer.name, server);
    } else {
      await mcpService.addServer(server);
    }
    setEditingServer(undefined);
    setNotice("MCP 服务器已保存");
    await refresh();
  }

  async function testServer(server: McpServerConfig) {
    setError(null);
    setNotice(null);
    setTestingName(server.name);
    try {
      const result: McpTestResult = await mcpService.testConnection(server.name);
      const status = await mcpService.getServerStatus(server.name);
      setStatuses((current) => ({ ...current, [server.name]: status }));
      setNotice(result.success ? `${server.name} 测试通过，发现 ${result.tools.length} 个工具` : `${server.name} 测试失败`);
      if (!result.success && result.error) setError(result.error);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setTestingName(null);
    }
  }

  async function toggleServer(server: McpServerConfig) {
    await mcpService.toggleServer(server.name, !server.active);
    await refresh();
  }

  async function deleteServer(server: McpServerConfig) {
    if (!window.confirm(`删除 MCP 服务器 ${server.name}？`)) return;
    await mcpService.removeServer(server.name);
    await refresh();
  }

  async function importServers(data: McpImportExport, scope: McpScope) {
    const result = await mcpService.importServers(data, scope);
    await refresh();
    return `导入 ${result.imported.length} 个，跳过 ${result.skipped.length} 个`;
  }

  async function exportServers(names: string[]) {
    return mcpService.exportServers(names);
  }

  function renderGroup(title: string, group: McpServerConfig[]) {
    if (!group.length) return null;
    return (
      <div className="space-y-3">
        <div className="text-center text-[11px] text-muted-foreground">-- {title} --</div>
        <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
          {group.map((server) => (
            <McpServerCard
              key={server.name}
              server={server}
              status={statuses[server.name]}
              testing={testingName === server.name}
              onDelete={deleteServer}
              onEdit={setEditingServer}
              onTest={(item) => void testServer(item)}
              onToggle={(item) => void toggleServer(item)}
            />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button variant="outline" onClick={() => void refresh()}>
              <RefreshCw className="h-4 w-4" aria-hidden="true" />
              刷新
            </Button>
            <Button variant="outline" onClick={() => setShowImportExport(true)}>
              <Upload className="h-4 w-4" aria-hidden="true" />
              导入/导出
            </Button>
            <Button onClick={() => setEditingServer(null)}>
              <Plus className="h-4 w-4" aria-hidden="true" />
              添加 MCP
            </Button>
          </>
        }
        description="管理 MCP 服务器配置、连接测试和工具发现结果"
        title="MCP 服务器"
      />

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard label="服务器" value={String(servers.length)} hint="用户与当前项目可见" />
        <StatCard label="最近通过" value={String(connectedCount)} hint="来自缓存测试状态" />
        <StatCard label="工具总数" value={String(totalTools)} hint={averageDuration ? `平均 ${averageDuration}ms` : "尚未测试"} />
      </div>

      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

      {visibleServers.length ? (
        <>
          {renderGroup("用户配置", userServers)}
          {renderGroup("项目配置", projectServers)}
        </>
      ) : (
        <SectionPanel title="MCP 服务器">
          <div className="flex min-h-40 flex-col items-center justify-center gap-3 text-center text-sm text-muted-foreground">
            <Boxes className="h-8 w-8" aria-hidden="true" />
            <div>当前没有可见的 MCP 服务器</div>
            <button className="text-primary underline-offset-4 hover:underline" onClick={() => setEditingServer(null)} type="button">
              添加第一个 MCP 服务器
            </button>
          </div>
        </SectionPanel>
      )}

      {editingServer !== undefined ? (
        <McpServerForm server={editingServer} onCancel={() => setEditingServer(undefined)} onSave={saveServer} />
      ) : null}
      {showImportExport ? (
        <McpImportExportModal
          servers={servers}
          onCancel={() => setShowImportExport(false)}
          onExport={exportServers}
          onImport={importServers}
        />
      ) : null}
    </div>
  );
}
