import { ScanSearch } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillOverlayPreview } from "../../../types/skill-overlay";
import { SkillOverlayDiffContent } from "./skill-overlay-diff-view";

export function SkillOverlayPreviewPanel({
  preview,
  matchCount,
  matchCountIncomplete,
}: {
  preview: SkillOverlayPreview;
  matchCount: number | null;
  matchCountIncomplete: boolean;
}) {
  const { t } = useTranslation();
  const witnesses = preview.witnesses;
  return <section aria-labelledby="skill-overlay-preview-result" className="space-y-3" aria-live="polite">
    <div className="rounded-md border border-border bg-muted/20 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="flex items-center gap-2 text-sm font-semibold" id="skill-overlay-preview-result">
          <ScanSearch className="h-4 w-4" />{t("skills.overlay.mutation.previewResult")}
        </h4>
        <Badge tone={preview.canCommit ? "success" : "warning"}>
          {t(preview.canCommit ? "skills.overlay.mutation.ready" : "skills.overlay.mutation.blocked")}
        </Badge>
      </div>
      <dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2">
        {matchCount !== null ? <Metric label={t("skills.overlay.mutation.matchCount")} value={`${matchCount}${matchCountIncomplete ? "+" : ""}`} /> : null}
        <Metric label={t("skills.overlay.mutation.tentativeRevision")} value={String(preview.tentativeRevision)} />
        <Metric label={t("skills.overlay.mutation.scan")} value={t(preview.scan.passed ? "skills.overlay.mutation.scanPassed" : "skills.overlay.mutation.scanFailed")} />
        <Metric label={t("skills.overlay.mutation.scannerVersion")} value={preview.scan.scannerVersion} />
      </dl>
      {preview.conflicts.length > 0 ? <ul className="mt-3 space-y-1 text-xs text-destructive">
        {preview.conflicts.map((conflict) => <li key={conflict.id}>{conflict.safeReason}</li>)}
      </ul> : null}
    </div>
    <div className="rounded-md border border-border bg-muted/10 p-3">
      <p className="text-xs font-semibold">{t("skills.overlay.mutation.expectedWitnesses")}</p>
      <dl className="mt-2 grid gap-2 text-[11px] sm:grid-cols-2">
        <Metric label={t("skills.overlay.mutation.overlayRevision")} value={witnesses.expectedOverlayRevision === null ? t("skills.overlay.mutation.newOverlay") : String(witnesses.expectedOverlayRevision)} />
        <Metric label={t("skills.overlay.baseInstructionHash")} value={shortHash(witnesses.expectedBaseInstructionHash)} />
        <Metric label={t("skills.overlay.basePackageHash")} value={shortHash(witnesses.expectedBasePackageHash)} />
        <Metric label={t("skills.overlay.pinned")} value={t(witnesses.expectedPinned ? "skills.overlay.mutation.yes" : "skills.overlay.mutation.no")} />
      </dl>
    </div>
    <div>
      <p className="mb-2 text-xs font-semibold">{t("skills.overlay.mutation.effectiveDiff")}</p>
      <SkillOverlayDiffContent diff={preview.diff} />
    </div>
  </section>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 rounded-md border border-border bg-background px-2.5 py-2">
    <dt className="text-muted-foreground">{label}</dt>
    <dd className="mt-1 truncate font-mono" title={value}>{value}</dd>
  </div>;
}

function shortHash(value: string) {
  return value.length > 18 ? `${value.slice(0, 10)}…${value.slice(-6)}` : value;
}
