import { KeyRound, Plus, RefreshCw, Server, Wifi } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { sshConnectionService } from "../../services/runtime-ssh-connection-client";
import { AsyncBoundary } from "../../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../../ui/async/async-view-state";
import type { MutationState } from "../../ui/async/mutation-state";
import { PageHeader } from "../../ui/page-header/PageHeader";
import type {
  SaveSshConnectionInput,
  SshConnection,
} from "../../types/ssh-connection";
import type { SettingsPageStatus } from "../settings-page-types";
import { StatCard } from "./page-parts";
import { SshConnectionCard } from "./ssh/ssh-connection-card";
import { SshConnectionForm } from "./ssh/ssh-connection-form";
import { errorMessage } from "./ssh/ssh-connection-utils";
import {
  refreshSshConnections,
  sshConnectionsQueryKey,
} from "./ssh/ssh-connection-query";

export function SshConnectionsPage({
  onStatusChange,
  searchTerm,
}: {
  onStatusChange?: (status: SettingsPageStatus | null) => void;
  searchTerm: string;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState<SshConnection | null | undefined>();
  const [notice, setNotice] = useState<string | null>(null);

  const connectionsQuery = useQuery({
    queryKey: sshConnectionsQueryKey,
    queryFn: () => sshConnectionService.listConnections(),
  });

  const saveMutation = useMutation({
    mutationFn: (input: SaveSshConnectionInput) =>
      editing
        ? sshConnectionService.updateConnection(editing.id, input)
        : sshConnectionService.createConnection(input),
    onSuccess: async () => {
      setEditing(undefined);
      setNotice(t("sshConnections.notice.saved"));
      await refreshSshConnections(queryClient);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (connectionId: string) =>
      sshConnectionService.deleteConnection(connectionId),
    onSuccess: () => refreshSshConnections(queryClient),
  });

  const testMutation = useMutation({
    mutationFn: (connectionId: string) =>
      sshConnectionService.testConnection(connectionId),
    onSuccess: async () => {
      setNotice(t("sshConnections.notice.testSucceeded"));
    },
    onSettled: () => refreshSshConnections(queryClient),
  });

  const connections = useMemo(
    () => connectionsQuery.data ?? [],
    [connectionsQuery.data],
  );
  const visibleConnections = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return connections;
    return connections.filter((connection) =>
      [
        connection.name,
        connection.host,
        connection.user,
        connection.defaultPath,
        connection.testStatus,
      ].some((value) => value.toLowerCase().includes(query)),
    );
  }, [connections, searchTerm]);
  const passwordCount = connections.filter(
    (connection) => connection.authMode === "password",
  ).length;
  const successCount = connections.filter(
    (connection) => connection.testStatus === "succeeded",
  ).length;

  // task 12.18: this page's own connectionsQuery projected into the shared AsyncBoundary's
  // AsyncViewState shape -- src/ui/ primitives cannot import this service's own error type
  // (ARCH-FE-005), so the projection lives here rather than in the primitive.
  const asyncState: AsyncViewState<SshConnection[]> = {
    data: connectionsQuery.data,
    error: connectionsQuery.isError
      ? { kind: "error", message: errorMessage(connectionsQuery.error), retryable: true }
      : undefined,
    initialLoading: connectionsQuery.isLoading,
    refreshing: connectionsQuery.isFetching && !connectionsQuery.isLoading,
    stale: connectionsQuery.isStale,
  };

  // Task 12.16: the same condition already rendered below, reported so this page's own nav
  // entry can flag it while the user looks at another page. Single condition, so no
  // pickPageStatus combination is needed.
  useEffect(() => {
    onStatusChange?.(connectionsQuery.isError ? { kind: "error", labelKey: "sshConnections.status.error" } : null);
    return () => onStatusChange?.(null);
  }, [connectionsQuery.isError, onStatusChange]);

  async function save(input: SaveSshConnectionInput) {
    setNotice(null);
    await saveMutation.mutateAsync(input).catch(() => undefined);
  }

  async function deleteConnection(connection: SshConnection) {
    setNotice(null);
    await deleteMutation.mutateAsync(connection.id).catch(() => undefined);
  }

  async function testConnection(connection: SshConnection) {
    setNotice(null);
    await testMutation.mutateAsync(connection.id).catch(() => undefined);
  }

  /** Task 12.18: projects this page's own single-in-flight `useMutation` (react-query already
   *  tracks `variables`/`isPending`/`error` for its own most recent call) into the shared
   *  `MutationState` shape, keyed to one connection at a time -- a second registry alongside
   *  `useMutation` would just be a second source of truth for the same fact. */
  function stateFor(mutation: typeof testMutation | typeof deleteMutation, connectionId: string): MutationState | undefined {
    if (mutation.variables !== connectionId) return undefined;
    if (mutation.isPending) return { pending: true, targetKey: connectionId };
    if (mutation.isError) {
      return { error: { kind: "error", message: errorMessage(mutation.error), retryable: true }, pending: false, targetKey: connectionId };
    }
    return undefined;
  }

  // Same message and action whether the list is genuinely empty or a search just matched
  // nothing, matching this page's own pre-existing behavior -- AsyncBoundary still picks the
  // right icon/variant (Inbox vs. SearchX) for each case on its own.
  const emptyStateSlot = {
    action: (
      <button
        className="text-primary underline-offset-4 hover:underline"
        onClick={() => setEditing(null)}
        type="button"
      >
        {t("sshConnections.emptyAction")}
      </button>
    ),
    title: t("sshConnections.empty"),
  };

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("sshConnections.description")}
        icon={KeyRound}
        moreMenuItems={[
          {
            icon: RefreshCw,
            id: "refresh",
            label: connectionsQuery.isFetching ? t("sshConnections.refreshing") : t("sshConnections.refresh"),
            onSelect: () => void connectionsQuery.refetch(),
          },
        ]}
        primaryAction={
          <Button onClick={() => setEditing(null)}>
            <Plus className="h-4 w-4" aria-hidden="true" />
            {t("sshConnections.add")}
          </Button>
        }
        title={t("sshConnections.title")}
      />

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard
          icon={Server}
          label={t("sshConnections.stats.total")}
          value={String(connections.length)}
          hint={t("sshConnections.stats.totalHint")}
        />
        <StatCard
          icon={KeyRound}
          label={t("sshConnections.stats.password")}
          value={String(passwordCount)}
          hint={t("sshConnections.stats.passwordHint")}
        />
        <StatCard
          icon={Wifi}
          label={t("sshConnections.stats.tested")}
          value={String(successCount)}
          hint={t("sshConnections.stats.testedHint")}
        />
      </div>

      {notice ? (
        <div className="rounded-md border p-3 text-sm ucd-status-success">
          {notice}
        </div>
      ) : null}

      <AsyncBoundary
        emptyState={emptyStateSlot}
        filtered={Boolean(searchTerm.trim())}
        filteredEmptyState={emptyStateSlot}
        isEmpty={() => visibleConnections.length === 0}
        onRetry={() => void connectionsQuery.refetch()}
        state={asyncState}
      >
        {() => (
          <div className="grid gap-3 lg:grid-cols-2 xl:grid-cols-3">
            {visibleConnections.map((connection) => (
              <SshConnectionCard
                connection={connection}
                deleteState={stateFor(deleteMutation, connection.id)}
                key={connection.id}
                onDelete={(item) => void deleteConnection(item)}
                onEdit={setEditing}
                onTest={(item) => void testConnection(item)}
                testState={stateFor(testMutation, connection.id)}
              />
            ))}
          </div>
        )}
      </AsyncBoundary>

      {editing !== undefined ? (
        <SshConnectionForm
          connection={editing}
          saving={saveMutation.isPending}
          onCancel={() => setEditing(undefined)}
          onSave={(input) => void save(input)}
        />
      ) : null}
    </div>
  );
}
