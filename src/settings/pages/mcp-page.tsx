import { Boxes, Cable, Plus, RefreshCw, Upload, Wrench } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { mcpService } from "../../services/runtime-mcp-client";
import { AsyncBoundary } from "../../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../../ui/async/async-view-state";
import type { MutationState } from "../../ui/async/mutation-state";
import { PageHeader } from "../../ui/page-header/PageHeader";
import type { McpScope, McpServerConfig } from "../../types/mcp";
import type { SettingsPageStatus } from "../settings-page-types";
import { StatCard } from "./page-parts";
import { McpImportExportModal } from "./mcp/mcp-import-export";
import { formatMcpFailure, mcpErrorFromUnknown, mcpMutationErrorMessage } from "./mcp/mcp-presentation";
import { McpScopeSection } from "./mcp/mcp-scope-section";
import { McpServerCard } from "./mcp/mcp-server-card";
import { McpServerForm } from "./mcp/mcp-server-form";
import { loadMcpServersAndStatuses, type McpServersAndStatuses, mcpServersQueryKey, refreshMcpServers } from "./mcp/mcp-server-query";
import { useMcpTestOperation } from "./mcp/use-mcp-test-operation";

const emptyServers: McpServerConfig[] = [];

export function McpPage({ onStatusChange, searchTerm }: { onStatusChange?: (status: SettingsPageStatus | null) => void; searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editingServer, setEditingServer] = useState<McpServerConfig | null | undefined>();
  const [showImportExport, setShowImportExport] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const serversQuery = useQuery({
    queryFn: loadMcpServersAndStatuses,
    queryKey: mcpServersQueryKey,
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
      setNotice(t("mcp.notice.saved"));
      await refreshMcpServers(queryClient);
    },
  });

  const toggleServerMutation = useMutation({
    mutationFn: (server: McpServerConfig) => mcpService.toggleServer(server.name, !server.active),
    onSuccess: () => refreshMcpServers(queryClient),
  });

  const deleteServerMutation = useMutation({
    mutationFn: (server: McpServerConfig) => mcpService.removeServer(server.name),
    onSuccess: () => refreshMcpServers(queryClient),
  });

  const importServersMutation = useMutation({
    mutationFn: ({ input, scope }: { input: string; scope: McpScope }) => mcpService.importServers(input, scope),
    onSuccess: () => refreshMcpServers(queryClient),
  });

  const { stateFor: testStateFor, testServer } = useMcpTestOperation({ onTestPassed: setNotice, t });

  const servers = serversQuery.data?.servers ?? emptyServers;
  const statuses = serversQuery.data?.statuses ?? {};

  // Task 12.16: the same error banner rendered below (save failures), plus the list query's own
  // failure now surfaced distinctly via AsyncBoundary -- both reported for the nav entry.
  // "mcp.status.error" already names a per-server test-result label -- this is the different,
  // page-level condition, so it uses its own "pageStatus" namespace instead of colliding.
  useEffect(() => {
    onStatusChange?.(error || serversQuery.isError ? { kind: "error", labelKey: "mcp.pageStatus.error" } : null);
    return () => onStatusChange?.(null);
  }, [error, onStatusChange, serversQuery.isError]);

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

  // Task 12.18: this page's own serversQuery projected into the shared AsyncBoundary's
  // AsyncViewState shape -- src/ui/ primitives cannot import this service's own error type
  // (ARCH-FE-005), so the projection lives here rather than in the primitive. A real,
  // previously-silent gap fixed as a side effect: a list-fetch failure used to just fall through
  // to the empty-state copy behind the page-level banner; AsyncBoundary now gives it a distinct
  // error+retry state, matching the same gap Extensions/Plugins found in their own migrations.
  const listFailure = mcpErrorFromUnknown(serversQuery.error);
  const asyncState: AsyncViewState<McpServersAndStatuses> = {
    data: serversQuery.data,
    error: serversQuery.isError
      ? { kind: "error", message: formatMcpFailure(t, listFailure.errorCode, listFailure.message), retryable: true }
      : undefined,
    initialLoading: serversQuery.isLoading,
    refreshing: serversQuery.isFetching && !serversQuery.isLoading,
    stale: serversQuery.isStale,
  };

  async function saveServer(server: McpServerConfig) {
    setError(null);
    await saveServerMutation.mutateAsync(server).catch((err) => {
      const failure = mcpErrorFromUnknown(err);
      setError(formatMcpFailure(t, failure.errorCode, failure.message));
    });
  }

  async function runTest(server: McpServerConfig) {
    setNotice(null);
    await testServer(server);
  }

  async function toggleServer(server: McpServerConfig) {
    await toggleServerMutation.mutateAsync(server).catch(() => undefined);
  }

  async function deleteServer(server: McpServerConfig) {
    await deleteServerMutation.mutateAsync(server).catch(() => undefined);
  }

  async function importServers(input: string, scope: McpScope) {
    return importServersMutation.mutateAsync({ input, scope });
  }

  async function exportServers(names: string[]) {
    return mcpService.exportServers(names);
  }

  /** Task 12.18: projects this page's own single-in-flight Toggle/Delete `useMutation`s (react-query
   *  already tracks `variables`/`isPending`/`error` for each one's own most recent call) into the
   *  shared `MutationState` shape, keyed to one server at a time -- matching SSH's own precedent,
   *  a registry alongside `useMutation` would just be a second source of truth for the same fact. */
  function mutationStateFor(mutation: typeof toggleServerMutation | typeof deleteServerMutation, serverName: string): MutationState | undefined {
    if (mutation.variables?.name !== serverName) return undefined;
    if (mutation.isPending) return { pending: true, targetKey: serverName };
    if (mutation.isError) {
      return { error: { kind: "error", message: mcpMutationErrorMessage(t, mutation.error), retryable: true }, pending: false, targetKey: serverName };
    }
    return undefined;
  }

  function renderServerCard(server: McpServerConfig) {
    return (
      <McpServerCard
        deleteState={mutationStateFor(deleteServerMutation, server.name)}
        key={server.name}
        onDelete={(item) => void deleteServer(item)}
        onEdit={setEditingServer}
        onTest={(item) => void runTest(item)}
        onToggle={(item) => void toggleServer(item)}
        server={server}
        status={statuses[server.name]}
        testState={testStateFor(server.name)}
        toggleState={mutationStateFor(toggleServerMutation, server.name)}
      />
    );
  }

  // Same message and action whether the list is genuinely empty or a search just matched
  // nothing, matching this page's own pre-existing behavior.
  const emptyStateSlot = {
    action: (
      <button className="text-primary underline-offset-4 hover:underline" onClick={() => setEditingServer(null)} type="button">
        {t("mcp.emptyAction")}
      </button>
    ),
    title: t("mcp.empty"),
  };

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("mcp.description")}
        icon={Boxes}
        moreMenuItems={[
          {
            icon: RefreshCw,
            id: "refresh",
            label: serversQuery.isFetching ? t("mcp.refreshing") : t("mcp.refresh"),
            onSelect: () => void serversQuery.refetch(),
          },
          {
            icon: Upload,
            id: "import-export",
            label: t("mcp.importExport"),
            onSelect: () => setShowImportExport(true),
          },
        ]}
        primaryAction={
          <Button onClick={() => setEditingServer(null)}>
            <Plus className="h-4 w-4" aria-hidden="true" />
            {t("mcp.add")}
          </Button>
        }
        title={t("mcp.title")}
      />

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard icon={Boxes} label={t("mcp.stats.servers")} value={String(servers.length)} hint={t("mcp.stats.serversHint")} />
        <StatCard icon={Cable} label={t("mcp.stats.connected")} value={String(connectedCount)} hint={t("mcp.stats.connectedHint")} />
        <StatCard icon={Wrench} label={t("mcp.stats.totalTools")} value={String(totalTools)} hint={averageDuration ? t("mcp.stats.average", { duration: averageDuration }) : t("mcp.stats.notTested")} />
      </div>

      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

      <AsyncBoundary
        emptyState={emptyStateSlot}
        filtered={Boolean(searchTerm.trim())}
        filteredEmptyState={emptyStateSlot}
        isEmpty={() => visibleServers.length === 0}
        onRetry={() => void serversQuery.refetch()}
        state={asyncState}
      >
        {() => (
          <div className="space-y-4">
            <McpScopeSection renderCard={renderServerCard} servers={userServers} title={t("mcp.group.user")} />
            <McpScopeSection renderCard={renderServerCard} servers={projectServers} title={t("mcp.group.project")} />
          </div>
        )}
      </AsyncBoundary>

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
