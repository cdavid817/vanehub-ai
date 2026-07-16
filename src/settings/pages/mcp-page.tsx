import { Boxes, Plus, RefreshCw, Upload } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { Button } from "../../components/ui/button";
import { mcpService } from "../../services/runtime-mcp-client";
import type { McpImportExport, McpScope, McpServerConfig, McpServerStatus, McpTestResult } from "../../types/mcp";
import { PageHeader, SectionPanel, StatCard } from "./page-parts";
import { McpImportExportModal } from "./mcp/mcp-import-export";
import { McpServerCard } from "./mcp/mcp-server-card";
import { McpServerForm } from "./mcp/mcp-server-form";

type StatusMap = Record<string, McpServerStatus>;

const mcpServersQueryKey = ["mcp", "servers"] as const;

async function loadMcpServersAndStatuses() {
  const servers = await mcpService.listServers();
  const entries = await Promise.all(
    servers.map(async (server) => [server.name, await mcpService.getServerStatus(server.name)] as const),
  );

  return {
    servers,
    statuses: Object.fromEntries(entries) as StatusMap,
  };
}

export function McpPage({ searchTerm }: { searchTerm: string }) {
  const queryClient = useQueryClient();
  const [editingServer, setEditingServer] = useState<McpServerConfig | null | undefined>();
  const [showImportExport, setShowImportExport] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const serversQuery = useQuery({
    queryKey: mcpServersQueryKey,
    queryFn: loadMcpServersAndStatuses,
  });

  const saveServerMutation = useMutation({
    mutationFn: async (server: McpServerConfig) => {
      if (editingServer?.name) {
        await mcpService.updateServer(editingServer.name, server);
      } else {
        await mcpService.addServer(server);
      }
    },
    onSuccess: async () => {
      setEditingServer(undefined);
      setNotice("MCP server saved");
      await queryClient.invalidateQueries({ queryKey: mcpServersQueryKey });
    },
  });

  const toggleServerMutation = useMutation({
    mutationFn: (server: McpServerConfig) => mcpService.toggleServer(server.name, !server.active),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mcpServersQueryKey }),
  });

  const deleteServerMutation = useMutation({
    mutationFn: (server: McpServerConfig) => mcpService.removeServer(server.name),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mcpServersQueryKey }),
  });

  const testServerMutation = useMutation({
    mutationFn: async (server: McpServerConfig) => ({
      server,
      result: await mcpService.testConnection(server.name),
    }),
    onSuccess: async ({ server, result }) => {
      setNotice(result.success ? `${server.name} test passed with ${result.tools.length} tools` : `${server.name} test failed`);
      if (!result.success && result.error) setError(result.error);
      await queryClient.invalidateQueries({ queryKey: mcpServersQueryKey });
    },
  });

  const importServersMutation = useMutation({
    mutationFn: ({ data, scope }: { data: McpImportExport; scope: McpScope }) => mcpService.importServers(data, scope),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: mcpServersQueryKey }),
  });

  const servers = serversQuery.data?.servers ?? [];
  const statuses = serversQuery.data?.statuses ?? {};
  const testingName = testServerMutation.isPending ? testServerMutation.variables?.name ?? null : null;
  const queryError = serversQuery.error instanceof Error ? serversQuery.error.message : serversQuery.error ? String(serversQuery.error) : null;
  const visibleError = error ?? queryError;

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
    setError(null);
    await saveServerMutation.mutateAsync(server).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function testServer(server: McpServerConfig) {
    setError(null);
    setNotice(null);
    await testServerMutation.mutateAsync(server).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function toggleServer(server: McpServerConfig) {
    setError(null);
    await toggleServerMutation.mutateAsync(server).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function deleteServer(server: McpServerConfig) {
    if (!window.confirm(`Delete MCP server ${server.name}?`)) return;
    setError(null);
    await deleteServerMutation.mutateAsync(server).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function importServers(data: McpImportExport, scope: McpScope) {
    const result = await importServersMutation.mutateAsync({ data, scope });
    return `Imported ${result.imported.length}, skipped ${result.skipped.length}`;
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
            <Button disabled={serversQuery.isFetching} variant="outline" onClick={() => void serversQuery.refetch()}>
              <RefreshCw className="h-4 w-4" aria-hidden="true" />
              {serversQuery.isFetching ? "Refreshing" : "Refresh"}
            </Button>
            <Button variant="outline" onClick={() => setShowImportExport(true)}>
              <Upload className="h-4 w-4" aria-hidden="true" />
              Import/Export
            </Button>
            <Button onClick={() => setEditingServer(null)}>
              <Plus className="h-4 w-4" aria-hidden="true" />
              Add MCP
            </Button>
          </>
        }
        description="Manage MCP server configuration, connection tests, and discovered tools"
        title="MCP Servers"
      />

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard label="Servers" value={String(servers.length)} hint="Visible in user and project scopes" />
        <StatCard label="Recently Passed" value={String(connectedCount)} hint="From cached test status" />
        <StatCard label="Total Tools" value={String(totalTools)} hint={averageDuration ? `Average ${averageDuration}ms` : "Not tested"} />
      </div>

      {visibleError ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{visibleError}</div> : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

      {serversQuery.isLoading ? (
        <SectionPanel title="MCP Servers">
          <div className="py-8 text-center text-sm text-muted-foreground">Loading MCP servers</div>
        </SectionPanel>
      ) : visibleServers.length ? (
        <>
          {renderGroup("User Configuration", userServers)}
          {renderGroup("Project Configuration", projectServers)}
        </>
      ) : (
        <SectionPanel title="MCP Servers">
          <div className="flex min-h-40 flex-col items-center justify-center gap-3 text-center text-sm text-muted-foreground">
            <Boxes className="h-8 w-8" aria-hidden="true" />
            <div>No visible MCP servers</div>
            <button className="text-primary underline-offset-4 hover:underline" onClick={() => setEditingServer(null)} type="button">
              Add the first MCP server
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
