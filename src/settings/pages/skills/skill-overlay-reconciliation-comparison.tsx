import { GitCompareArrows } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillOverlayBoundedText } from "../../../types/skill-overlay";
import type { SkillOverlayReconciliationBaseSnapshot, SkillOverlayReconciliationPreview } from "../../../types/skill-overlay-reconciliation";
import { SkillOverlayDiffContent } from "./skill-overlay-diff-view";

export function SkillOverlayReconciliationComparison({ preview, previewCurrent }: {
  preview: SkillOverlayReconciliationPreview;
  previewCurrent: boolean;
}) {
  const { t } = useTranslation();
  return <section aria-labelledby="skill-overlay-three-way" aria-live="polite" className="space-y-4">
    <div className="flex flex-wrap items-center justify-between gap-2">
      <div><h4 className="flex items-center gap-2 text-sm font-semibold" id="skill-overlay-three-way"><GitCompareArrows className="h-4 w-4" />{t("skills.overlay.reconcile.threeWayTitle")}</h4><p className="mt-1 text-xs text-muted-foreground">{t("skills.overlay.reconcile.threeWayDescription")}</p></div>
      <Badge tone={previewCurrent && preview.canCommit && preview.finalDiffComplete ? "success" : "warning"}>{t(previewCurrent ? preview.canCommit ? "skills.overlay.reconcile.previewReady" : "skills.overlay.reconcile.previewBlocked" : "skills.overlay.reconcile.previewStale")}</Badge>
    </div>
    <div className="grid gap-3 xl:grid-cols-3">
      <BaseColumn snapshot={preview.witnessedBase} title={t("skills.overlay.reconcile.witnessedBase")} />
      <BaseColumn snapshot={preview.currentBase} title={t("skills.overlay.reconcile.currentBase")} />
      <ResultColumn preview={preview} />
    </div>
    <div className="rounded-md border border-border bg-muted/10 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2"><h5 className="text-xs font-semibold">{t("skills.overlay.reconcile.finalDiff")}</h5><Badge tone={preview.finalDiffComplete ? "success" : "warning"}>{t(preview.finalDiffComplete ? "skills.overlay.reconcile.complete" : "skills.overlay.reconcile.incomplete")}</Badge></div>
      <SkillOverlayDiffContent diff={preview.finalDiff} />
    </div>
    <dl className="grid gap-2 text-[11px] sm:grid-cols-2 lg:grid-cols-4">
      <Metric label={t("skills.overlay.mutation.overlayRevision")} value={String(preview.witnesses.expectedOverlayRevision ?? 0)} />
      <Metric label={t("skills.overlay.baseInstructionHash")} value={preview.witnesses.expectedBaseInstructionHash} />
      <Metric label={t("skills.overlay.basePackageHash")} value={preview.witnesses.expectedBasePackageHash} />
      <Metric label={t("skills.overlay.pinned")} value={t(preview.witnesses.expectedPinned ? "skills.overlay.mutation.yes" : "skills.overlay.mutation.no")} />
    </dl>
  </section>;
}

function BaseColumn({ snapshot, title }: { snapshot: SkillOverlayReconciliationBaseSnapshot; title: string }) {
  const { t } = useTranslation();
  return <article className="min-w-0 rounded-md border border-border bg-muted/10 p-3"><h5 className="text-xs font-semibold">{title}</h5><dl className="mt-2 space-y-1 text-[11px]"><Row label={t("skills.overlay.reconcile.identity")} value={snapshot.baseIdentity} /><Row label={t("skills.overlay.reconcile.layer")} value={t(`skills.layer.${snapshot.baseLayer}`)} /><Row label={t("skills.overlay.reconcile.instructionHash")} value={snapshot.instructionHash} /><Row label={t("skills.overlay.reconcile.packageHash")} value={snapshot.packageHash} /></dl><InstructionContent value={snapshot.instructions} /></article>;
}

function ResultColumn({ preview }: { preview: SkillOverlayReconciliationPreview }) {
  const { t } = useTranslation();
  return <article className="min-w-0 rounded-md border border-primary/40 bg-primary/5 p-3"><h5 className="text-xs font-semibold">{t("skills.overlay.reconcile.proposedEffective")}</h5><dl className="mt-2 space-y-1 text-[11px]"><Row label={t("skills.overlay.effectiveHash")} value={preview.proposedEffective.effectiveHash} /><Row label={t("skills.overlay.resources")} value={String(preview.proposedEffective.resources.length)} /></dl><InstructionContent value={preview.proposedEffective.instructions} /></article>;
}

function InstructionContent({ value }: { value: SkillOverlayBoundedText | null }) {
  const { t } = useTranslation();
  if (!value) return <p className="mt-3 rounded border border-dashed border-border p-2 text-xs text-muted-foreground">{t("skills.overlay.reconcile.witnessedContentUnavailable")}</p>;
  return <div className="mt-3"><p className="mb-1 text-[11px] text-muted-foreground">{t("skills.overlay.reconcile.instructions")}</p><pre className="max-h-52 overflow-auto whitespace-pre-wrap break-words rounded border border-border bg-background p-2 font-mono text-xs leading-5">{value.content}</pre>{value.truncated ? <p className="mt-1 text-[11px] text-warning">{t("skills.overlay.diff.contentTruncated")}</p> : null}</div>;
}

function Row({ label, value }: { label: string; value: string }) { return <div><dt className="text-muted-foreground">{label}</dt><dd className="break-all font-mono">{value}</dd></div>; }
function Metric({ label, value }: { label: string; value: string }) { return <div className="min-w-0 rounded-md border border-border bg-background px-2.5 py-2"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>; }
