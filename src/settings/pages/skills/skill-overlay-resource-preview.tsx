import { FileCheck2, ScanSearch } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillOverlayPreview, SkillOverlayResourceSummary } from "../../../types/skill-overlay";

export function SkillOverlayResourcePreview({
  preview,
  logicalPath,
  mediaType,
  sizeBytes,
  replacing,
  existing,
}: {
  preview: SkillOverlayPreview;
  logicalPath: string;
  mediaType: string;
  sizeBytes: number;
  replacing: boolean;
  existing?: SkillOverlayResourceSummary;
}) {
  const { t } = useTranslation();
  return <section aria-labelledby="skill-overlay-resource-preview" aria-live="polite" className="space-y-3">
    <div className="rounded-md border border-border bg-muted/20 p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="flex items-center gap-2 text-sm font-semibold" id="skill-overlay-resource-preview">
          <ScanSearch className="h-4 w-4" />{t("skills.overlay.resource.previewTitle")}
        </h4>
        <Badge tone={preview.canCommit ? "success" : "warning"}>
          {t(preview.canCommit ? "skills.overlay.mutation.ready" : "skills.overlay.mutation.blocked")}
        </Badge>
      </div>
      <dl className="mt-3 grid gap-2 text-xs sm:grid-cols-2">
        <Metric label={t("skills.overlay.resource.operation")} value={t(replacing ? "skills.overlay.resource.replace" : "skills.overlay.resource.add")} />
        <Metric label={t("skills.overlay.resource.path")} value={logicalPath} />
        <Metric label={t("skills.overlay.resource.mediaType")} value={mediaType} />
        <Metric label={t("skills.overlay.resource.size")} value={formatBytes(sizeBytes)} />
        <Metric label={t("skills.overlay.mutation.tentativeRevision")} value={String(preview.tentativeRevision)} />
        <Metric label={t("skills.overlay.mutation.scannerVersion")} value={preview.scan.scannerVersion} />
        <Metric label={t("skills.overlay.mutation.scan")} value={t(preview.scan.passed ? "skills.overlay.mutation.scanPassed" : "skills.overlay.mutation.scanFailed")} />
      </dl>
      <p className="mt-3 flex items-start gap-2 text-xs leading-5 text-muted-foreground">
        <FileCheck2 className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
        {existing
          ? t("skills.overlay.resource.shadowPreview", { count: existing.shadowed.length })
          : t("skills.overlay.resource.noShadowPreview")}
      </p>
    </div>
    <div className="rounded-md border border-border bg-muted/10 p-3 text-xs">
      <p className="font-semibold">{t("skills.overlay.mutation.expectedWitnesses")}</p>
      <p className="mt-2 text-muted-foreground">
        {t("skills.overlay.resource.witnessSummary", {
          revision: preview.witnesses.expectedOverlayRevision ?? t("skills.overlay.mutation.newOverlay"),
          payload: preview.witnesses.expectedPayloadHash ? shortHash(preview.witnesses.expectedPayloadHash) : t("skills.overlay.resource.noPayloadWitness"),
        })}
      </p>
    </div>
  </section>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 rounded-md border border-border bg-background px-2.5 py-2">
    <dt className="text-muted-foreground">{label}</dt>
    <dd className="mt-1 break-all font-mono">{value}</dd>
  </div>;
}

export function formatBytes(value: number) {
  return value < 1024 ? `${value} B` : value < 1_048_576 ? `${(value / 1024).toFixed(1)} KiB` : `${(value / 1_048_576).toFixed(2)} MiB`;
}

function shortHash(value: string) {
  return value.length > 18 ? `${value.slice(0, 10)}…${value.slice(-6)}` : value;
}
