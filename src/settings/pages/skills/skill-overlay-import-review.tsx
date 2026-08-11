import { FileCheck2, ScanSearch, ShieldAlert } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillOverlayImportReview, SkillOverlayWitnesses } from "../../../types/skill-overlay";
import { SkillOverlayDiffContent } from "./skill-overlay-diff-view";
import { formatBytes } from "./skill-overlay-resource-preview";

export function SkillOverlayImportReviewPanel({
  review,
  witnesses,
}: {
  review: SkillOverlayImportReview;
  witnesses: SkillOverlayWitnesses | null;
}) {
  const { t } = useTranslation();
  return <section aria-labelledby="skill-overlay-import-review" aria-live="polite" className="space-y-4">
    <div className="rounded-md border border-warning/40 bg-warning/10 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="flex items-center gap-2 text-sm font-semibold" id="skill-overlay-import-review"><ShieldAlert className="h-4 w-4" />{t("skills.overlay.import.reviewTitle")}</h4>
        <Badge tone="warning">{t("skills.overlay.import.untrusted")}</Badge>
      </div>
      <p className="mt-2 text-xs leading-5 text-muted-foreground">{t("skills.overlay.import.reviewDescription")}</p>
      <dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2">
        <Metric label={t("skills.overlay.import.safeSource")} value={review.sourceSummary} />
        <Metric label={t("skills.overlay.import.reviewedRevision")} value={String(review.revision)} />
        <Metric label={t("skills.overlay.import.documentHash")} value={review.documentHash} />
        <Metric label={t("skills.overlay.import.scannerVersion")} value={review.scan.scannerVersion} />
        <Metric label={t("skills.overlay.import.diffBaseHash")} value={review.diff.baseHash} />
        <Metric label={t("skills.overlay.import.diffEffectiveHash")} value={review.diff.effectiveHash} />
      </dl>
    </div>

    <div className="rounded-md border border-border bg-muted/10 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h5 className="flex items-center gap-2 text-xs font-semibold"><ScanSearch className="h-4 w-4" />{t("skills.overlay.import.scanTitle")}</h5>
        <Badge tone={review.scan.passed ? "success" : "danger"}>{t(review.scan.passed ? "skills.overlay.mutation.scanPassed" : "skills.overlay.mutation.scanFailed")}</Badge>
      </div>
      <p className="mt-2 text-xs text-muted-foreground">{review.scan.safeRuleIds.length > 0 ? review.scan.safeRuleIds.join(", ") : t("skills.overlay.import.noScanFindings")}</p>
      {review.scan.ruleIdsTruncated ? <p className="mt-1 text-xs text-warning">{t("skills.overlay.import.listTruncated")}</p> : null}
    </div>

    {witnesses ? <div className="rounded-md border border-border bg-muted/10 p-3">
      <h5 className="text-xs font-semibold">{t("skills.overlay.import.exactWitnesses")}</h5>
      <dl className="mt-2 grid gap-2 text-[11px] sm:grid-cols-2">
        <Metric label={t("skills.overlay.import.reviewedRevision")} value={String(review.revision)} />
        <Metric label={t("skills.overlay.import.documentHash")} value={review.documentHash} />
        <Metric label={t("skills.overlay.baseInstructionHash")} value={witnesses.expectedBaseInstructionHash} />
        <Metric label={t("skills.overlay.basePackageHash")} value={witnesses.expectedBasePackageHash} />
        <Metric label={t("skills.overlay.import.payloadWitness")} value={witnesses.expectedPayloadHash ?? t("skills.overlay.resource.noPayloadWitness")} />
        <Metric label={t("skills.overlay.pinned")} value={t(witnesses.expectedPinned ? "skills.overlay.mutation.yes" : "skills.overlay.mutation.no")} />
      </dl>
    </div> : null}

    <ReviewLists review={review} />
    <div>
      <h5 className="mb-2 text-xs font-semibold">{t("skills.overlay.import.diffTitle")}</h5>
      <SkillOverlayDiffContent diff={review.diff} />
    </div>
  </section>;
}

function ReviewLists({ review }: { review: SkillOverlayImportReview }) {
  const { t } = useTranslation();
  return <div className="grid gap-3 lg:grid-cols-3">
    <ReviewList icon={<FileCheck2 className="h-4 w-4" />} title={t("skills.overlay.import.mutationsTitle")} truncated={review.mutationsTruncated}>
      {review.mutations.map((mutation) => <li className="rounded border border-border bg-background px-2 py-1.5" key={mutation.id}>
        <span className="font-medium">{t(`skills.overlay.mutations.${mutation.kind}`)}</span><span className="ml-2 text-muted-foreground">{t(`skills.overlay.scope.${mutation.scope}`)} · {mutation.state}</span>
      </li>)}
    </ReviewList>
    <ReviewList icon={<FileCheck2 className="h-4 w-4" />} title={t("skills.overlay.import.filesTitle")} truncated={review.resourcesTruncated}>
      {review.resources.map((resource) => <li className="rounded border border-border bg-background px-2 py-1.5" key={`${resource.mutationId}-${resource.logicalPath}`}>
        <p className="break-all font-mono">{resource.logicalPath}</p><p className="mt-1 text-muted-foreground">{resource.mediaType} · {formatBytes(resource.sizeBytes)}</p><p className="mt-1 break-all text-muted-foreground">{resource.contentHash}</p>
      </li>)}
    </ReviewList>
    <ReviewList icon={<ShieldAlert className="h-4 w-4" />} title={t("skills.overlay.import.conflictsTitle")} truncated={review.conflictsTruncated}>
      {review.conflicts.map((conflict) => <li className="rounded border border-border bg-background px-2 py-1.5" key={conflict.id}><p>{conflict.safeReason}</p><p className="mt-1 text-muted-foreground">{conflict.state}</p></li>)}
    </ReviewList>
  </div>;
}

function ReviewList({ icon, title, truncated, children }: { icon: ReactNode; title: string; truncated: boolean; children: ReactNode }) {
  const { t } = useTranslation();
  const values = Array.isArray(children) ? children : [children];
  return <section className="min-w-0 rounded-md border border-border bg-muted/10 p-3">
    <h5 className="flex items-center gap-2 text-xs font-semibold">{icon}{title}</h5>
    {values.length > 0 ? <ul className="mt-2 space-y-2 text-xs">{children}</ul> : <p className="mt-2 text-xs text-muted-foreground">{t("skills.overlay.import.none")}</p>}
    {truncated ? <p className="mt-2 text-xs text-warning">{t("skills.overlay.import.listTruncated")}</p> : null}
  </section>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 rounded-md border border-border bg-background px-2.5 py-2"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>;
}
