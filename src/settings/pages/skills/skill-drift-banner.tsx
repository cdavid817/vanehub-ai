import { AlertTriangle, RefreshCw, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillDriftReport, SkillSyncResult } from "../../../types/skill";

export function SkillDriftBanner({
  drift,
  syncResult,
  syncing,
  syncError,
  onSync,
  onDismiss,
}: {
  drift: SkillDriftReport | null;
  syncResult: SkillSyncResult | null;
  syncing: boolean;
  syncError?: string | null;
  onSync: () => void;
  onDismiss?: () => void;
}) {
  const { t } = useTranslation();
  const issues = drift?.issues ?? [];
  if (!drift && !syncResult && !syncError) {
    return <div className="inline-flex w-fit rounded-full border px-3 py-1 text-xs text-muted-foreground">{t("skills.drift.loading")}</div>;
  }
  if (issues.length === 0 && !syncResult && !syncError) {
    return <div className="inline-flex w-fit rounded-full border px-3 py-1 text-xs ucd-status-success">{t("skills.drift.inSync")}</div>;
  }

  return (
    <div className="rounded-lg border px-4 py-3 ucd-status-warning">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-medium">
          <AlertTriangle className="h-4 w-4" aria-hidden="true" />
          {syncError ? t("skills.drift.syncFailed") : issues.length > 0 ? t("skills.drift.issuesDetected", { count: issues.length }) : t("skills.drift.syncCompleted")}
        </div>
        <div className="flex items-center gap-1">
          <Button disabled={syncing || issues.length === 0} onClick={onSync} variant="outline">
            <RefreshCw className="h-4 w-4" aria-hidden="true" />
            {t("skills.drift.sync")}
          </Button>
          {(syncResult || syncError) && onDismiss ? <Button aria-label={t("skills.drift.dismiss")} onClick={onDismiss} size="icon" variant="ghost"><X className="h-4 w-4" /></Button> : null}
        </div>
      </div>
      {syncError ? <p className="mt-2 text-xs text-destructive" role="alert">{syncError}</p> : null}
      {issues.length > 0 ? (
        <ul className="mt-2 space-y-1 text-xs">
          {issues.slice(0, 4).map((issue) => (
            <li key={`${issue.skillId}:${issue.type}:${issue.agentId ?? ""}`}>{issue.skillId}: {issue.message}</li>
          ))}
        </ul>
      ) : null}
      {syncResult ? (
        <div className="mt-2 text-xs">
          {t("skills.drift.syncSummary", {
            backedUp: syncResult.backedUp.length,
            mounted: syncResult.mounted.length,
            restored: syncResult.restored.length,
            overwritten: syncResult.overwritten.length,
            failed: syncResult.failed.length,
          })}
        </div>
      ) : null}
    </div>
  );
}
