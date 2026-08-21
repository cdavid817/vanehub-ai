import { useEffect, useState } from "react";
import { Copy, Loader2, Pencil, Play, Power, PowerOff, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import {
  useDeleteLoopDefinitionMutation,
  useDuplicateLoopDefinitionMutation,
  useSetLoopEnabledMutation,
} from "../hooks/use-loop-mutations";
import type { LoopDefinition, LoopRun } from "../types/loop";

const activeStatuses: LoopRun["status"][] = ["queued", "running", "paused", "awaiting-acceptance"];

export function LoopDefinitionOverview({
  definition,
  onDeleted,
  onEdit,
  onPreflight,
  runs,
}: {
  definition: LoopDefinition;
  onDeleted: () => void;
  onEdit: () => void;
  onPreflight: () => void;
  runs: LoopRun[];
}) {
  const { i18n, t } = useTranslation();
  const toggle = useSetLoopEnabledMutation();
  const duplicate = useDuplicateLoopDefinitionMutation();
  const remove = useDeleteLoopDefinitionMutation();
  const [confirmation, setConfirmation] = useState<"duplicate" | "delete" | null>(null);
  const [duplicateName, setDuplicateName] = useState("");
  const activeRun = runs.find((run) => activeStatuses.includes(run.status));
  const pending = toggle.isPending || duplicate.isPending || remove.isPending;
  const error = toggle.error ?? duplicate.error ?? remove.error;

  useEffect(() => {
    setConfirmation(null);
    setDuplicateName(t("loops.definition.copyName", { name: definition.name }));
  }, [definition.id, definition.name, t]);

  async function duplicateDefinition() {
    const name = duplicateName.trim();
    if (!name || name === definition.name) return;
    await duplicate.mutateAsync({ definition, name });
    setConfirmation(null);
  }

  async function deleteDefinition() {
    await remove.mutateAsync(definition.id);
    setConfirmation(null);
    onDeleted();
  }

  return (
    <article className="mx-auto grid w-full max-w-4xl gap-5" aria-labelledby="loop-definition-title">
      <header className="flex flex-col gap-3 border-b border-border/70 pb-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="text-lg font-semibold" id="loop-definition-title">{definition.name}</h2>
            <span className={definition.enabled ? "text-xs font-medium text-success" : "text-xs font-medium text-muted-foreground"}>{t(definition.enabled ? "loops.definition.enabled" : "loops.definition.disabled")}</span>
          </div>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">{definition.goal}</p>
        </div>
        <Button disabled={!definition.enabled || Boolean(activeRun)} onClick={onPreflight} size="sm" type="button"><Play aria-hidden="true" />{t("loops.definition.start")}</Button>
      </header>

      <div className="flex flex-wrap gap-2" aria-label={t("loops.definition.actions")}>
        <Button disabled={pending} onClick={onEdit} size="sm" type="button" variant="outline"><Pencil aria-hidden="true" />{t("loops.definition.edit")}</Button>
        <Button disabled={pending || Boolean(activeRun)} onClick={() => toggle.mutate({ definition, enabled: !definition.enabled })} size="sm" type="button" variant="outline">
          {definition.enabled ? <PowerOff aria-hidden="true" /> : <Power aria-hidden="true" />}{t(definition.enabled ? "loops.definition.disable" : "loops.definition.enable")}
        </Button>
        <Button disabled={pending} onClick={() => setConfirmation("duplicate")} size="sm" type="button" variant="outline"><Copy aria-hidden="true" />{t("loops.definition.duplicate")}</Button>
        <Button className="text-destructive hover:text-destructive" disabled={pending || Boolean(activeRun)} onClick={() => setConfirmation("delete")} size="sm" type="button" variant="outline"><Trash2 aria-hidden="true" />{t("loops.definition.delete")}</Button>
      </div>

      {activeRun ? <p className="rounded-md border border-warning/40 bg-warning/5 px-3 py-2 text-xs text-warning">{t("loops.definition.activeGuard", { status: t(`loops.status.${activeRun.status}`) })}</p> : null}
      {confirmation === "duplicate" ? <Confirmation title={t("loops.definition.duplicateTitle")} description={t("loops.definition.duplicateDescription")} onCancel={() => setConfirmation(null)} onConfirm={() => void duplicateDefinition()} pending={duplicate.isPending}>
        <label className="grid gap-1.5"><span className="text-xs font-medium text-muted-foreground">{t("loops.editor.field.name")}</span><input className="ucd-input h-9 rounded px-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setDuplicateName(event.target.value)} value={duplicateName} /></label>
      </Confirmation> : null}
      {confirmation === "delete" ? <Confirmation title={t("loops.definition.deleteTitle")} description={t("loops.definition.deleteDescription")} onCancel={() => setConfirmation(null)} onConfirm={() => void deleteDefinition()} pending={remove.isPending} /> : null}
      {pending ? <p aria-live="polite" className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />{t("loops.definition.pending")}</p> : null}
      {error ? <p aria-live="assertive" className="text-xs text-destructive">{error instanceof Error ? error.message : String(error)}</p> : null}

      <OverviewSection title={t("loops.definition.scope")}>
        <Value label={t("loops.editor.field.project")} value={definition.projectPath} />
        <Value label={t("loops.editor.field.branch")} value={definition.baseBranch} />
        <ListValue label={t("loops.editor.field.allowedPaths")} values={definition.allowedPaths} />
        <ListValue label={t("loops.editor.field.protectedPaths")} values={definition.protectedPaths} />
      </OverviewSection>
      <OverviewSection title={t("loops.definition.acceptance")}><ListValue label={t("loops.editor.field.acceptance")} values={definition.acceptanceCriteria} /></OverviewSection>
      <OverviewSection title={t("loops.definition.roles")}>
        <Value label={t("loops.editor.field.worker")} value={definition.workerAgentId} />
        <Value label={t("loops.editor.field.verifier")} value={definition.verifierAgentId} />
      </OverviewSection>
      <OverviewSection title={t("loops.definition.policy")}>
        <ListValue label={t("loops.editor.field.commands")} values={definition.verificationCommands.map((command) => `${command.program} ${command.args.join(" ")}`.trim())} />
        <Value label={t("loops.definition.limits")} value={t("loops.definition.limitSummary", { iterations: definition.limits.maxIterations, step: definition.limits.stepTimeoutSeconds, total: definition.limits.totalTimeoutSeconds })} />
        <Value label={t("loops.definition.humanGate")} value={t("loops.definition.humanGateDescription")} />
      </OverviewSection>
      <OverviewSection title={t("loops.definition.recentRuns")}>
        {runs.length === 0 ? <p className="text-xs text-muted-foreground">{t("loops.definition.noRuns")}</p> : runs.slice(0, 3).map((run) => <Value key={run.id} label={new Date(run.createdAt).toLocaleString(i18n.resolvedLanguage)} value={t(`loops.status.${run.status}`)} />)}
      </OverviewSection>
    </article>
  );
}

function OverviewSection({ children, title }: { children: React.ReactNode; title: string }) {
  return <section className="grid gap-3 border-b border-border/60 pb-5 last:border-0"><h3 className="text-xs font-semibold uppercase text-muted-foreground">{title}</h3><dl className="grid gap-3 sm:grid-cols-2">{children}</dl></section>;
}

function Value({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-1 wrap-break-word text-sm">{value || "—"}</dd></div>;
}

function ListValue({ label, values }: { label: string; values: string[] }) {
  return <div className="min-w-0"><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-1 text-sm">{values.length ? <ul className="list-inside list-disc">{values.map((value) => <li className="wrap-break-word" key={value}>{value}</li>)}</ul> : "—"}</dd></div>;
}

function Confirmation({ children, description, onCancel, onConfirm, pending, title }: { children?: React.ReactNode; description: string; onCancel: () => void; onConfirm: () => void; pending: boolean; title: string }) {
  const { t } = useTranslation();
  return <section className="grid gap-3 rounded-md border border-warning/50 bg-warning/5 p-3" role="alertdialog" aria-label={title}><div><h3 className="text-sm font-medium">{title}</h3><p className="mt-1 text-xs text-muted-foreground">{description}</p></div>{children}<div className="flex justify-end gap-2"><Button disabled={pending} onClick={onCancel} size="sm" type="button" variant="ghost">{t("loops.controls.dismiss")}</Button><Button disabled={pending} onClick={onConfirm} size="sm" type="button">{t("loops.controls.confirmAction")}</Button></div></section>;
}
