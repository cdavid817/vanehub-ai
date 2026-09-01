import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { CliConfigDriftState, CliConfigProfile } from "../../../types/cli-agent-config";

export type CliConfigPendingAction =
  | { kind: "import" }
  | { kind: "apply"; profile: CliConfigProfile }
  | { kind: "delete"; profile: CliConfigProfile };

export interface CliConfigActionDecision {
  importName?: string;
  confirmAuthFileReplacement?: boolean;
}

export function CliConfigActionDialog({ action, driftState, pending, onClose, onConfirm }: {
  action: CliConfigPendingAction;
  driftState: CliConfigDriftState;
  pending: boolean;
  onClose: () => void;
  onConfirm: (decision: CliConfigActionDecision) => void;
}) {
  const { t } = useTranslation();
  const [importName, setImportName] = useState(t("agents.globalConfig.importDefaultName"));
  const isApply = action.kind === "apply";
  const replacesAuth = isApply && action.profile.payload.kind === "codex-cli" && action.profile.payload.authStrategy === "replace-auth";
  const title = t(`agentConfigurations.dialog.${action.kind}.title`);
  const description = action.kind === "import"
    ? t("agentConfigurations.dialog.import.description")
    : t(`agentConfigurations.dialog.${action.kind}.description`, { name: action.profile.name });

  return (
    <ApplicationDialog closeDisabled={pending} description={description} maxWidth="max-w-lg" onClose={onClose} title={title}>
      {action.kind === "import" ? <label className="grid gap-1.5 text-sm">{t("agents.globalConfig.profileName")}<input data-dialog-autofocus className="ucd-input h-10 rounded-md px-3" onChange={(event) => setImportName(event.target.value)} value={importName} /></label> : null}
      {isApply && (driftState === "drifted" || driftState === "missing") ? <p className="rounded-md border p-3 text-sm ucd-status-warning">{t("agentConfigurations.dialog.apply.autoBackfill")}</p> : null}
      {replacesAuth ? <p className="mt-3 rounded-md border p-3 text-sm ucd-status-warning">{t("agents.globalConfig.confirmAuthReplace")}</p> : null}
      {action.kind === "delete" && action.profile.appliedState !== "saved" ? <p className="rounded-md border p-3 text-sm ucd-status-warning">{t("agentConfigurations.dialog.delete.appliedWarning")}</p> : null}
      <div className="mt-6 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
        <Button disabled={pending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
        <Button data-dialog-autofocus={action.kind !== "import" ? true : undefined} disabled={pending || (action.kind === "import" && !importName.trim())} onClick={() => onConfirm({ importName: action.kind === "import" ? importName.trim() : undefined, confirmAuthFileReplacement: replacesAuth })} variant={action.kind === "delete" ? "destructive" : "default"}>
          {pending ? t("agentConfigurations.dialog.pending") : t(`agentConfigurations.dialog.${action.kind}.confirm`)}
        </Button>
      </div>
    </ApplicationDialog>
  );
}
