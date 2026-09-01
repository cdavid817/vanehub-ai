import { Pencil, Trash2, Wifi } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionMenu, type ActionMenuItem } from "../../../ui/actions/ActionMenu";
import { MutationStatus } from "../../../ui/async/MutationStatus";
import type { MutationState } from "../../../ui/async/mutation-state";
import { CopyDiagnosticsButton } from "../../../ui/diagnostics/CopyDiagnosticsButton";
import { StatusBadge, type StatusTone } from "../../../ui/status/StatusBadge";
import type { SshConnection, SshConnectionTestStatus } from "../../../types/ssh-connection";
import { buildSshConnectionDiagnosticFields } from "./ssh-connection-diagnostic-summary";

const statusTone: Record<SshConnectionTestStatus, StatusTone> = {
  "not-tested": "neutral",
  succeeded: "success",
  failed: "danger",
};

export function SshConnectionCard({
  connection,
  deleteState,
  testState,
  onDelete,
  onEdit,
  onTest,
}: {
  connection: SshConnection;
  deleteState: MutationState | undefined;
  testState: MutationState | undefined;
  onDelete: (connection: SshConnection) => void;
  onEdit: (connection: SshConnection) => void;
  onTest: (connection: SshConnection) => void;
}) {
  const { t } = useTranslation();
  const items: ActionMenuItem[] = [
    {
      disabled: testState?.pending,
      icon: Wifi,
      id: "test",
      label: testState?.pending ? t("sshConnections.testing") : t("sshConnections.test"),
      onSelect: () => onTest(connection),
    },
    {
      icon: Pencil,
      id: "edit",
      label: t("sshConnections.edit"),
      onSelect: () => onEdit(connection),
    },
    {
      confirmation: { title: t("sshConnections.confirm.delete", { name: connection.name }) },
      disabled: deleteState?.pending,
      icon: Trash2,
      id: "delete",
      label: t("sshConnections.delete"),
      onSelect: () => onDelete(connection),
      tone: "destructive",
    },
  ];

  return (
    <article className="ucd-panel ucd-interactive grid gap-3 rounded-lg p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="truncate text-sm font-semibold">{connection.name}</h3>
          <p className="mt-1 truncate text-xs text-muted-foreground">
            {connection.user}@{connection.host}:{connection.port}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <StatusBadge label={t(`sshConnections.status.${connection.testStatus}`)} tone={statusTone[connection.testStatus]} />
          <ActionMenu items={items} triggerLabel={t("sshConnections.rowActions", { name: connection.name })} />
        </div>
      </div>
      <div className="grid gap-1 text-xs text-muted-foreground">
        <div className="truncate">{connection.defaultPath}</div>
        <div>
          {connection.authMode === "password"
            ? t("sshConnections.auth.password")
            : t("sshConnections.auth.key")}
        </div>
      </div>
      {connection.lastError ? (
        <div className="rounded border p-2 text-xs ucd-status-danger">
          {connection.lastError}
        </div>
      ) : null}
      <MutationStatus state={testState} />
      <MutationStatus state={deleteState} />
      <div className="flex justify-end">
        <CopyDiagnosticsButton fields={buildSshConnectionDiagnosticFields(connection, t)} />
      </div>
    </article>
  );
}
