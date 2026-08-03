import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { OnePieceProviderProfile } from "../../../types/agent";

export type OnePieceProfileAction = { kind: "activate" | "delete"; profile: OnePieceProviderProfile };

export function OnePieceProfileActionDialog({ action, pending, onClose, onConfirm }: {
  action: OnePieceProfileAction;
  pending: boolean;
  onClose: () => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  return (
    <ApplicationDialog closeDisabled={pending} description={t(`onepiece.action.${action.kind}.description`, { name: action.profile.name })} maxWidth="max-w-lg" onClose={onClose} title={t(`onepiece.action.${action.kind}.title`)}>
      {action.kind === "delete" && action.profile.active ? <p className="rounded-md border p-3 text-sm ucd-status-warning">{t("onepiece.action.delete.activeWarning")}</p> : null}
      <div className="mt-6 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
        <Button disabled={pending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
        <Button data-dialog-autofocus disabled={pending} onClick={onConfirm}>{pending ? t("agentConfigurations.dialog.pending") : t(`onepiece.action.${action.kind}.confirm`)}</Button>
      </div>
    </ApplicationDialog>
  );
}
