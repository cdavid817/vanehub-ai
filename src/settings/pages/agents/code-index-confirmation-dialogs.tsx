import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { CodeIndexWorkspace } from "../../../types/code-index";

export function CodeEmbeddingConfirmationDialog({
  workspace,
  profileId,
  model,
  pending,
  onClose,
  onConfirm,
}: {
  workspace: CodeIndexWorkspace;
  profileId: string;
  model: string;
  pending: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  const [acknowledged, setAcknowledged] = useState(false);
  return (
    <ApplicationDialog closeDisabled={pending} description={t("codeIndex.confirmation.description")} onClose={onClose} title={t("codeIndex.confirmation.title")}>
      <dl className="grid gap-3 rounded-md border border-border bg-muted/20 p-4 text-sm sm:grid-cols-4">
        <div><dt className="text-xs text-muted-foreground">{t("codeIndex.confirmation.workspace")}</dt><dd className="mt-1 font-medium">{workspace.displayName}</dd></div>
        <div><dt className="text-xs text-muted-foreground">{t("codeIndex.confirmation.provider")}</dt><dd className="mt-1 break-all font-medium">{profileId}</dd></div>
        <div><dt className="text-xs text-muted-foreground">{t("codeIndex.confirmation.model")}</dt><dd className="mt-1 break-all font-medium">{model}</dd></div>
        <div><dt className="text-xs text-muted-foreground">{t("codeIndex.confirmation.chunks")}</dt><dd className="mt-1 font-medium tabular-nums">{workspace.status.totalChunks}</dd></div>
      </dl>
      <p className="mt-4 text-sm">{t("codeIndex.confirmation.estimate", { count: workspace.status.estimatedEmbeddingRequests })}</p>
      <label className="mt-4 flex items-start gap-3 rounded-md border border-border p-3 text-sm leading-5">
        <input checked={acknowledged} className="mt-0.5 h-4 w-4 accent-primary" onChange={(event) => setAcknowledged(event.target.checked)} type="checkbox" />
        <span>{t("codeIndex.confirmation.acknowledge")}</span>
      </label>
      <div className="mt-5 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
        <Button disabled={pending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
        <Button data-dialog-autofocus disabled={!acknowledged || pending} onClick={onConfirm}>{pending ? t("agentConfigurations.dialog.pending") : t("codeIndex.confirmation.confirm")}</Button>
      </div>
    </ApplicationDialog>
  );
}

export function CodeIndexDestructiveDialog({ workspace, action, pending, onClose, onConfirm }: {
  workspace: CodeIndexWorkspace;
  action: "rebuild" | "delete";
  pending: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  return (
    <ApplicationDialog closeDisabled={pending} description={t(`codeIndex.${action}.description`, { name: workspace.displayName })} onClose={onClose} title={t(`codeIndex.${action}.title`)}>
      <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
        <Button disabled={pending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
        <Button data-dialog-autofocus disabled={pending} onClick={onConfirm}>{pending ? t("agentConfigurations.dialog.pending") : t(`codeIndex.${action}.confirm`)}</Button>
      </div>
    </ApplicationDialog>
  );
}
