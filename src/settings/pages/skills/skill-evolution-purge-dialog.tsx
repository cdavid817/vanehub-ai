import { useMutation } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";

export function SkillEvolutionPurgeDialog({
  skillId,
  workspace,
  returnFocus,
  onClose,
  onPurged,
}: {
  skillId: string;
  workspace?: string;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onPurged: () => Promise<unknown>;
}) {
  const { t } = useTranslation();
  const mutation = useMutation({
    mutationFn: () => agentService.purgeSkillEvolutionEvidence({
      operationId: crypto.randomUUID(),
      workspace,
      skillId,
      confirmed: true,
    }),
    onSuccess: async () => {
      await onPurged();
      onClose();
    },
  });
  return <ApplicationDialog
    closeDisabled={mutation.isPending}
    description={t("skills.evolution.purgeDescription", { skillId })}
    maxWidth="max-w-lg"
    onClose={onClose}
    returnFocus={returnFocus}
    title={t("skills.evolution.purgeTitle")}
  >
    <div className="space-y-4 text-sm">
      <div className="rounded-lg border border-destructive/30 bg-destructive/5 p-3">
        <p className="font-medium">{t("skills.evolution.purgeRemoves")}</p>
        <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-muted-foreground">
          <li>{t("skills.evolution.purgeSignals")}</li>
          <li>{t("skills.evolution.purgeSeeds")}</li>
          <li>{t("skills.evolution.purgeFeedback")}</li>
        </ul>
      </div>
      <p className="text-xs leading-5 text-muted-foreground">{t("skills.evolution.purgePreserves")}</p>
      {mutation.isError ? <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-destructive" role="alert">{t("skills.evolution.purgeError")}</p> : null}
      <div className="flex justify-end gap-2 border-t border-border pt-4">
        <Button disabled={mutation.isPending} onClick={onClose} variant="outline">{t("skills.evolution.cancel")}</Button>
        <Button data-dialog-autofocus disabled={mutation.isPending} onClick={() => mutation.mutate()} variant="destructive">
          {t(mutation.isPending ? "skills.evolution.purging" : "skills.evolution.confirmPurge")}
        </Button>
      </div>
    </div>
  </ApplicationDialog>;
}
