import { CircleAlert, GitCommitHorizontal, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { SkillOverlayHistoryEntry } from "../../../types/skill-overlay";

export function SkillOverlayHistoryEntryCard({ entry }: { entry: SkillOverlayHistoryEntry }) {
  const { i18n, t } = useTranslation();
  const trustKey = entry.action === "import" ? "untrusted" : entry.action === "promote" ? "trusted" : "unchanged";
  const conflictKey = entry.action === "conflict" ? "recorded" : entry.action === "reconcile" ? "resolved" : "none";

  return <li className="relative pl-7">
    <span aria-hidden className="absolute left-0 top-1.5 flex h-5 w-5 items-center justify-center rounded-full border border-primary/40 bg-background text-primary">
      {entry.action === "conflict" ? <CircleAlert className="h-3 w-3" /> : <GitCommitHorizontal className="h-3 w-3" />}
    </span>
    <article className="rounded-md border border-border bg-background p-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h6 className="text-sm font-semibold">{t(`skills.overlay.history.action.${entry.action}`)}</h6>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("skills.overlay.history.revisionTransition", {
              from: entry.priorRevision == null ? "∅" : entry.priorRevision,
              to: entry.nextRevision,
            })}
          </p>
        </div>
        <time className="text-xs text-muted-foreground" dateTime={entry.timestamp}>{formatTimestamp(entry.timestamp, i18n.language)}</time>
      </div>
      <div className="mt-3 flex flex-wrap gap-1.5">
        <Badge tone="muted">{t(`skills.overlay.history.actor.${entry.actor}`)}</Badge>
        <Badge tone="muted">{t(`skills.overlay.scope.${entry.scope}`)}</Badge>
        <Badge tone={trustKey === "trusted" ? "success" : trustKey === "untrusted" ? "warning" : "muted"}>
          <ShieldCheck className="mr-1 h-3 w-3" />{t(`skills.overlay.history.trust.${trustKey}`)}
        </Badge>
        <Badge tone={conflictKey === "recorded" ? "danger" : conflictKey === "resolved" ? "success" : "muted"}>
          {t(`skills.overlay.history.conflict.${conflictKey}`)}
        </Badge>
      </div>
      <div className="mt-3 rounded-md border border-border bg-muted/20 p-2.5">
        <p className="text-[11px] font-medium text-muted-foreground">{t("skills.overlay.history.safeDiffSummary")}</p>
        <p className="mt-1 break-words text-xs">{entry.safeOutcome}</p>
      </div>
      <details className="mt-3 text-xs">
        <summary className="min-h-11 cursor-pointer select-none py-3 font-medium text-primary sm:min-h-9 sm:py-2">
          {t("skills.overlay.history.integrityEvidence")}
        </summary>
        <dl className="grid gap-2 rounded-md border border-border bg-muted/10 p-3 sm:grid-cols-2">
          <Evidence label={t("skills.overlay.history.eventId")} value={entry.eventId} />
          <Evidence label={t("skills.overlay.history.scannerVersion")} value={entry.scannerVersion} />
          <Evidence label={t("skills.overlay.history.priorDocumentHash")} value={entry.priorDocumentHash ?? "∅"} />
          <Evidence label={t("skills.overlay.history.nextDocumentHash")} value={entry.nextDocumentHash} />
          <Evidence label={t("skills.overlay.history.priorEventHash")} value={entry.priorEventHash ?? "∅"} />
          <Evidence label={t("skills.overlay.history.eventHash")} value={entry.eventHash} />
        </dl>
      </details>
    </article>
  </li>;
}

function Evidence({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>;
}

function formatTimestamp(value: string, language: string) {
  const timestamp = new Date(value);
  if (Number.isNaN(timestamp.getTime())) return value;
  return new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" }).format(timestamp);
}
