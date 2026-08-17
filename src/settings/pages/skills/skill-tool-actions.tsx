import { useMutation } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillToolRevision } from "../../../types/skill-tools";
import { SkillToolTrustDialog } from "./skill-tool-trust-dialog";

type ConfirmedOperation = "revoke" | "quarantine";

export function SkillToolActions({ onRefresh, tool }: { onRefresh: () => Promise<unknown>; tool: SkillToolRevision }) {
  const { t } = useTranslation();
  const [trustTrigger, setTrustTrigger] = useState<HTMLElement | null>(null);
  const [confirmation, setConfirmation] = useState<{ kind: ConfirmedOperation; trigger: HTMLElement } | null>(null);
  const mutation = useMutation({
    mutationFn: (operation: "validate" | "enable" | "disable" | "recover") => {
      if (operation === "validate") return agentService.validateSkillToolRevision({ revision: tool.revision });
      if (operation === "recover") return agentService.recoverSkillTool({ revision: tool.revision });
      return agentService.setSkillToolEnabled({ revision: tool.revision, enabled: operation === "enable" });
    },
    onSuccess: () => onRefresh(),
  });
  const blocked = mutation.isPending || tool.runtimeSupport === "unsupported-web-runtime";
  return <>
    <div className="mt-3 flex flex-wrap gap-2 border-t border-border pt-3">
      <Button disabled={blocked} onClick={() => mutation.mutate("validate")} size="sm" variant="outline">{t("skills.tools.validateAction")}</Button>
      <Button disabled={blocked} onClick={(event) => setTrustTrigger(event.currentTarget)} size="sm" variant="outline">{t(tool.trusted ? "skills.tools.retrust" : "skills.tools.trustAction")}</Button>
      {tool.trusted ? <Button disabled={blocked} onClick={(event) => setConfirmation({ kind: "revoke", trigger: event.currentTarget })} size="sm" variant="outline">{t("skills.tools.revokeAction")}</Button> : null}
      <Button disabled={blocked || !tool.trusted || tool.validation !== "valid" || tool.quarantined} onClick={() => mutation.mutate(tool.enabled ? "disable" : "enable")} size="sm">{t(tool.enabled ? "skills.tools.disableAction" : "skills.tools.enableAction")}</Button>
      {tool.quarantined ? <Button disabled={blocked} onClick={() => mutation.mutate("recover")} size="sm" variant="outline">{t("skills.tools.recoverAction")}</Button> : <Button disabled={blocked} onClick={(event) => setConfirmation({ kind: "quarantine", trigger: event.currentTarget })} size="sm" variant="outline">{t("skills.tools.quarantineAction")}</Button>}
    </div>
    {tool.runtimeSupport === "unsupported-web-runtime" ? <p className="mt-2 text-xs text-muted-foreground" role="note">{t("skills.tools.webMutationUnsupported")}</p> : null}
    {mutation.isError ? <p className="mt-2 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive" role="alert">{mutation.error.message}</p> : null}
    {trustTrigger ? <SkillToolTrustDialog onClose={() => setTrustTrigger(null)} onUpdated={onRefresh} returnFocus={trustTrigger} tool={tool} /> : null}
    {confirmation ? <SkillToolConfirmationDialog kind={confirmation.kind} onClose={() => setConfirmation(null)} onUpdated={onRefresh} returnFocus={confirmation.trigger} tool={tool} /> : null}
  </>;
}

function SkillToolConfirmationDialog({ kind, onClose, onUpdated, returnFocus, tool }: { kind: ConfirmedOperation; onClose: () => void; onUpdated: () => Promise<unknown>; returnFocus: HTMLElement; tool: SkillToolRevision }) {
  const { t } = useTranslation();
  const mutation = useMutation({
    mutationFn: () => kind === "revoke"
      ? agentService.setSkillToolTrust({ revision: tool.revision, trusted: false, actor: "settings-user" })
      : agentService.quarantineSkillTool({ revision: tool.revision, reason: "manual-security-review" }),
    onSuccess: async () => { await onUpdated(); onClose(); },
  });
  return <ApplicationDialog closeDisabled={mutation.isPending} description={t(`skills.tools.confirm.${kind}.description`)} maxWidth="max-w-lg" onClose={onClose} returnFocus={returnFocus} title={t(`skills.tools.confirm.${kind}.title`)}>
    <div className="space-y-4 text-sm"><p className="break-all rounded-md bg-muted/40 p-3 font-mono text-xs">{tool.canonicalId}<br />{tool.revision}</p>{mutation.isError ? <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-destructive" role="alert">{mutation.error.message}</p> : null}<div className="flex justify-end gap-2 border-t border-border pt-4"><Button disabled={mutation.isPending} onClick={onClose} variant="outline">{t("skills.tools.cancel")}</Button><Button className="bg-destructive text-destructive-foreground hover:bg-destructive/90" data-dialog-autofocus disabled={mutation.isPending} onClick={() => mutation.mutate()}>{t(`skills.tools.confirm.${kind}.action`)}</Button></div></div>
  </ApplicationDialog>;
}
