import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../components/ui/application-dialog";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { CliToolStatus } from "../../types/agent";
import { CliInstallationList } from "./cli-installation-list";

interface CliConflictDialogProps {
  tool: CliToolStatus | null;
  onCancel: () => void;
  onConfirm: () => void;
}

export function CliConflictDialog({ tool, onCancel, onConfirm }: CliConflictDialogProps) {
  const { t } = useTranslation();
  if (!tool) return null;

  return (
    <ApplicationDialog
      description={t("cli.confirm.description")}
      footer={(
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel}>{t("cli.confirm.cancel")}</Button>
          <Button data-dialog-autofocus onClick={onConfirm}>{t("cli.confirm.continue")}</Button>
        </div>
      )}
      maxWidth="max-w-xl"
      onClose={onCancel}
      title={t("cli.confirm.title", { name: tool.displayName })}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-[hsl(var(--warning))]" aria-hidden="true" />
        <CliInstallationList installations={tool.installations} />
      </div>
      <div className="mt-4 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3 text-xs">
        <div className="text-muted-foreground">{t("cli.confirm.target")}</div>
        <div className="mt-1 break-all font-mono">
          {tool.activeInstallationPath ? normalizeDisplayPath(tool.activeInstallationPath) : t("cli.notAvailable")}
        </div>
      </div>
    </ApplicationDialog>
  );
}
