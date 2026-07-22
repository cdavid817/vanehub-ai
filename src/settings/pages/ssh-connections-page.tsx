import { KeyRound, Plus, RefreshCw, Server, Wifi } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { sshConnectionService } from "../../services/runtime-ssh-connection-client";
import type {
  SaveSshConnectionInput,
  SshConnection,
} from "../../types/ssh-connection";
import { PageHeader, SectionPanel, StatCard } from "./page-parts";
import { SshConnectionCard } from "./ssh/ssh-connection-card";
import { SshConnectionForm } from "./ssh/ssh-connection-form";
import { errorMessage } from "./ssh/ssh-connection-utils";
import {
  refreshSshConnections,
  sshConnectionsQueryKey,
} from "./ssh/ssh-connection-query";

export function SshConnectionsPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState<SshConnection | null | undefined>();
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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
  const visibleError =
    error ??
    (connectionsQuery.error instanceof Error
      ? connectionsQuery.error.message
      : null);

  async function save(input: SaveSshConnectionInput) {
    setError(null);
    setNotice(null);
    await saveMutation
      .mutateAsync(input)
      .catch((saveError) => setError(errorMessage(saveError)));
  }

  async function deleteConnection(connection: SshConnection) {
    if (
      !window.confirm(
        t("sshConnections.confirm.delete", { name: connection.name }),
      )
    )
      return;
    setError(null);
    setNotice(null);
    await deleteMutation
      .mutateAsync(connection.id)
      .catch((deleteError) => setError(errorMessage(deleteError)));
  }

  async function testConnection(connection: SshConnection) {
    setError(null);
    setNotice(null);
    await testMutation
      .mutateAsync(connection.id)
      .catch((testError) => setError(errorMessage(testError)));
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button
              disabled={connectionsQuery.isFetching}
              variant="outline"
              onClick={() => void connectionsQuery.refetch()}
            >
              <RefreshCw className="h-4 w-4" aria-hidden="true" />
              {connectionsQuery.isFetching
                ? t("sshConnections.refreshing")
                : t("sshConnections.refresh")}
            </Button>
            <Button onClick={() => setEditing(null)}>
              <Plus className="h-4 w-4" aria-hidden="true" />
              {t("sshConnections.add")}
            </Button>
          </>
        }
        description={t("sshConnections.description")}
        icon={KeyRound}
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

      {visibleError ? (
        <div className="rounded-md border p-3 text-sm ucd-status-danger">
          {visibleError}
        </div>
      ) : null}
      {notice ? (
        <div className="rounded-md border p-3 text-sm ucd-status-success">
          {notice}
        </div>
      ) : null}

      {connectionsQuery.isLoading ? (
        <SectionPanel title={t("sshConnections.title")}>
          <div className="py-8 text-center text-sm text-muted-foreground">
            {t("sshConnections.loading")}
          </div>
        </SectionPanel>
      ) : visibleConnections.length ? (
        <div className="grid gap-3 lg:grid-cols-2 xl:grid-cols-3">
          {visibleConnections.map((connection) => (
            <SshConnectionCard
              connection={connection}
              key={connection.id}
              testing={
                testMutation.isPending &&
                testMutation.variables === connection.id
              }
              onDelete={(item) => void deleteConnection(item)}
              onEdit={setEditing}
              onTest={(item) => void testConnection(item)}
            />
          ))}
        </div>
      ) : (
        <SectionPanel title={t("sshConnections.title")}>
          <div className="flex min-h-40 flex-col items-center justify-center gap-3 text-center text-sm text-muted-foreground">
            <KeyRound className="h-8 w-8" aria-hidden="true" />
            <div>{t("sshConnections.empty")}</div>
            <button
              className="text-primary underline-offset-4 hover:underline"
              onClick={() => setEditing(null)}
              type="button"
            >
              {t("sshConnections.emptyAction")}
            </button>
          </div>
        </SectionPanel>
      )}

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
