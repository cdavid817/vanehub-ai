import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Settings2, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorPolicy } from "../../../types/skill-curator";

export function SkillCuratorPolicyPanel({ service, workspaceId }: { service: SkillCuratorService; workspaceId: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: ["skill-curator-policy", workspaceId],
    queryFn: () => service.getSkillCuratorPolicy(workspaceId),
  });
  const policy = query.data?.ok ? query.data.value : undefined;
  const [draft, setDraft] = useState<CuratorPolicy | null>(null);
  useEffect(() => { if (policy) setDraft(policy); }, [policy]);
  const save = useMutation({
    mutationFn: () => {
      if (!draft || !policy) throw new Error("policy_unavailable");
      const values = {
        enqueueRoutes: draft.enqueueRoutes,
        requireRejectionReason: draft.requireRejectionReason,
        requireDeferReason: draft.requireDeferReason,
        maximumDeferDays: draft.maximumDeferDays,
        openRetentionDays: draft.openRetentionDays,
        terminalRetentionDays: draft.terminalRetentionDays,
        notificationsEnabled: draft.notificationsEnabled,
        digestEnabled: draft.digestEnabled,
        draftDisplayLimitBytes: draft.draftDisplayLimitBytes,
        diffDisplayLimitBytes: draft.diffDisplayLimitBytes,
      };
      return service.updateSkillCuratorPolicy({ workspaceId, expectedRevision: policy.revision, policy: values });
    },
    onSuccess: async (result) => {
      if (!result.ok) return;
      await Promise.all([
        query.refetch(),
        queryClient.invalidateQueries({ queryKey: ["skill-curator-candidate"] }),
        queryClient.invalidateQueries({ queryKey: ["skill-curator-queue"] }),
      ]);
    },
  });
  const error = save.data && !save.data.ok ? save.data.error : undefined;
  return <details className="rounded-xl border border-border bg-background p-4"><summary className="flex min-h-9 cursor-pointer list-none items-center gap-2 text-sm font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><Settings2 className="h-4 w-4 text-primary" />{t("skills.curator.policy.title")}</summary>
    <p className="mt-2 text-xs leading-5 text-muted-foreground">{t("skills.curator.policy.description")}</p>
    {query.isLoading ? <p className="mt-3 text-xs text-muted-foreground" role="status">{t("skills.curator.policy.loading")}</p> : null}
    {query.isError || (query.data && !query.data.ok) ? <p className="mt-3 text-xs text-destructive" role="alert">{t("skills.curator.policy.loadError")}</p> : null}
    {draft ? <div className="mt-4 space-y-4"><div className="grid gap-3 sm:grid-cols-2"><Toggle checked={draft.notificationsEnabled} label={t("skills.curator.policy.notifications")} onChange={(notificationsEnabled) => setDraft({ ...draft, notificationsEnabled })} /><Toggle checked={draft.digestEnabled} label={t("skills.curator.policy.digest")} onChange={(digestEnabled) => setDraft({ ...draft, digestEnabled })} /><NumberField label={t("skills.curator.policy.maxDefer")} max={180} onChange={(maximumDeferDays) => setDraft({ ...draft, maximumDeferDays })} value={draft.maximumDeferDays} /><NumberField label={t("skills.curator.policy.openRetention")} max={180} onChange={(openRetentionDays) => setDraft({ ...draft, openRetentionDays })} value={draft.openRetentionDays} /><NumberField label={t("skills.curator.policy.terminalRetention")} max={365} onChange={(terminalRetentionDays) => setDraft({ ...draft, terminalRetentionDays })} value={draft.terminalRetentionDays} /></div>
      <div className="flex gap-2 rounded-md border border-primary/20 bg-primary/5 p-3 text-xs leading-5 text-muted-foreground"><ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-primary" />{t("skills.curator.policy.manualOnly")}</div>
      {error ? <p className="text-xs text-destructive" role="alert">{t("skills.curator.policy.error", { code: error.reasonCode ?? error.code })}</p> : null}
      <div className="flex justify-end"><Button disabled={save.isPending} onClick={() => save.mutate()} size="sm">{save.isPending ? t("skills.curator.policy.saving") : t("skills.curator.policy.save")}</Button></div>
    </div> : null}
  </details>;
}

function Toggle({ checked, label, onChange }: { checked: boolean; label: string; onChange: (value: boolean) => void }) { return <label className="flex min-h-11 items-center justify-between gap-3 rounded-md border border-border p-3 text-xs"><span>{label}</span><input checked={checked} className="h-4 w-4 accent-primary" onChange={(event) => onChange(event.target.checked)} type="checkbox" /></label>; }
function NumberField({ label, max, onChange, value }: { label: string; max: number; onChange: (value: number) => void; value: number }) { return <label className="text-xs text-muted-foreground"><span>{label}</span><input className="mt-1 h-9 w-full rounded-md border border-border bg-background px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" max={max} min={1} onChange={(event) => onChange(event.target.valueAsNumber)} type="number" value={value} /></label>; }
