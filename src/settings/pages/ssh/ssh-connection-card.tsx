import { Pencil, Trash2, Wifi } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { SshConnection, SshConnectionTestStatus } from "../../../types/ssh-connection";

const statusTone: Record<SshConnectionTestStatus, "success" | "danger" | "muted"> = {
  "not-tested": "muted",
  succeeded: "success",
  failed: "danger",
};

export function SshConnectionCard({
  connection,
  testing,
  onDelete,
  onEdit,
  onTest,
}: {
  connection: SshConnection;
  testing: boolean;
  onDelete: (connection: SshConnection) => void;
  onEdit: (connection: SshConnection) => void;
  onTest: (connection: SshConnection) => void;
}) {
  const { t } = useTranslation();
  return (
    <article className="ucd-panel ucd-interactive grid gap-3 rounded-lg p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="truncate text-sm font-semibold">{connection.name}</h3>
          <p className="mt-1 truncate text-xs text-muted-foreground">
            {connection.user}@{connection.host}:{connection.port}
          </p>
        </div>
        <Badge tone={statusTone[connection.testStatus]}>
          {t(`sshConnections.status.${connection.testStatus}`)}
        </Badge>
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
      <div className="flex flex-wrap justify-end gap-2">
        <Button className="h-8 px-3 text-xs" disabled={testing} onClick={() => onTest(connection)} type="button" variant="outline">
          <Wifi className="h-3.5 w-3.5" aria-hidden="true" />
          {testing ? t("sshConnections.testing") : t("sshConnections.test")}
        </Button>
        <Button className="h-8 px-3 text-xs" onClick={() => onEdit(connection)} type="button" variant="outline">
          <Pencil className="h-3.5 w-3.5" aria-hidden="true" />
          {t("sshConnections.edit")}
        </Button>
        <Button className="h-8 px-3 text-xs" onClick={() => onDelete(connection)} type="button" variant="outline">
          <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
          {t("sshConnections.delete")}
        </Button>
      </div>
    </article>
  );
}
