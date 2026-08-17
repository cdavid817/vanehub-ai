import { useMutation } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillToolRevision } from "../../../types/skill-tools";

export function SkillToolTrustDialog({ onClose, onUpdated, returnFocus, tool }: {
  onClose: () => void;
  onUpdated: () => Promise<unknown>;
  returnFocus: HTMLElement | null;
  tool: SkillToolRevision;
}) {
  const { t } = useTranslation();
  const mutation = useMutation({
    mutationFn: () => agentService.setSkillToolTrust({ revision: tool.revision, trusted: true, actor: "settings-user" }),
    onSuccess: async () => { await onUpdated(); onClose(); },
  });
  const diff = tool.capabilityDiff;
  return <ApplicationDialog closeDisabled={mutation.isPending} description={t("skills.tools.trust.description")} maxWidth="max-w-xl" onClose={onClose} returnFocus={returnFocus} title={t(tool.trusted ? "skills.tools.trust.retrustTitle" : "skills.tools.trust.title")}>
    <div className="space-y-4 text-sm">
      <dl className="grid gap-2 sm:grid-cols-2">
        <TrustFact label={t("skills.tools.sourceScope")} value={t(`skills.tools.scope.${tool.sourceScope}`)} />
        <TrustFact label={t("skills.tools.validationResult")} value={t(`skills.tools.validation.${tool.validation}`)} />
        <TrustFact label={t("skills.tools.baseRevision")} mono value={tool.baseRevision} />
        <TrustFact label={t("skills.tools.manifestHash")} mono value={tool.manifestHash} />
        <TrustFact label={t("skills.tools.implementationHash")} mono value={tool.implementationHash} />
        <TrustFact label={t("skills.tools.capabilityDigest")} mono value={tool.capabilityDigest} />
      </dl>
      <section aria-labelledby="skill-tool-capability-diff" className="rounded-lg border border-border p-3">
        <h4 className="font-semibold" id="skill-tool-capability-diff">{t("skills.tools.capabilityDiff")}</h4>
        {!diff ? <p className="mt-2 text-xs text-muted-foreground">{t("skills.tools.capabilityDiffUnavailable")}</p> : <div className="mt-2 grid gap-3 sm:grid-cols-2"><CapabilityList label={t("skills.tools.capabilityAdded")} values={diff.added} /><CapabilityList label={t("skills.tools.capabilityRemoved")} values={diff.removed} /></div>}
      </section>
      {tool.validation !== "valid" ? <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">{t("skills.tools.trust.validationRequired")}</p> : null}
      {mutation.isError ? <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">{mutation.error.message}</p> : null}
      <div className="flex justify-end gap-2 border-t border-border pt-4"><Button disabled={mutation.isPending} onClick={onClose} variant="outline">{t("skills.tools.cancel")}</Button><Button data-dialog-autofocus disabled={mutation.isPending || tool.validation !== "valid"} onClick={() => mutation.mutate()}>{t(mutation.isPending ? "skills.tools.trust.saving" : "skills.tools.trust.confirm")}</Button></div>
    </div>
  </ApplicationDialog>;
}

function TrustFact({ label, mono, value }: { label: string; mono?: boolean; value: string }) {
  return <div className="min-w-0 rounded-md bg-muted/40 p-2"><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className={`mt-1 break-all text-xs ${mono ? "font-mono" : "font-medium"}`}>{value}</dd></div>;
}

function CapabilityList({ label, values }: { label: string; values: string[] }) {
  const { t } = useTranslation();
  return <div><h5 className="text-xs font-medium">{label}</h5>{values.length ? <ul className="mt-1 list-disc space-y-1 pl-4 font-mono text-[11px]">{values.map((value) => <li className="break-all" key={value}>{value}</li>)}</ul> : <p className="mt-1 text-[11px] text-muted-foreground">{t("skills.tools.none")}</p>}</div>;
}
