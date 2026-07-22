import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 p-4" role="presentation">
      <section
        aria-describedby="cli-conflict-description"
        aria-labelledby="cli-conflict-title"
        aria-modal="true"
        className="ucd-panel max-h-[80vh] w-full max-w-xl overflow-y-auto rounded-lg border border-border p-4 shadow-xl"
        role="dialog"
      >
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-[hsl(var(--warning))]" aria-hidden="true" />
          <div>
            <h2 className="font-semibold" id="cli-conflict-title">{t("cli.confirm.title", { name: tool.displayName })}</h2>
            <p className="mt-1 text-sm text-muted-foreground" id="cli-conflict-description">
              {t("cli.confirm.description")}
            </p>
          </div>
        </div>
        <div className="mt-4">
          <CliInstallationList installations={tool.installations} />
        </div>
        <div className="mt-4 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3 text-xs">
          <div className="text-muted-foreground">{t("cli.confirm.target")}</div>
          <div className="mt-1 break-all font-mono">
            {tool.activeInstallationPath ? normalizeDisplayPath(tool.activeInstallationPath) : t("cli.notAvailable")}
          </div>
        </div>
        <div className="mt-4 flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel}>{t("cli.confirm.cancel")}</Button>
          <Button onClick={onConfirm}>{t("cli.confirm.continue")}</Button>
        </div>
      </section>
    </div>
  );
}
