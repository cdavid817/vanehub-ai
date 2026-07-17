import { AlertTriangle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillDriftReport, SkillSyncResult } from "../../../types/skill";

export function SkillDriftBanner({
  drift,
  syncResult,
  syncing,
  onSync,
}: {
  drift: SkillDriftReport | null;
  syncResult: SkillSyncResult | null;
  syncing: boolean;
  onSync: () => void;
}) {
  const { t } = useTranslation();
  const issues = drift?.issues ?? [];
  if (issues.length === 0 && !syncResult) {
    return <div className="rounded-lg border px-4 py-3 text-sm ucd-status-success">{t("skills.drift.inSync")}</div>;
  }

  return (
    <div className="rounded-lg border px-4 py-3 ucd-status-warning">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2 text-sm font-medium">
          <AlertTriangle className="h-4 w-4" aria-hidden="true" />
          {issues.length > 0 ? t("skills.drift.issuesDetected", { count: issues.length }) : t("skills.drift.syncCompleted")}
        </div>
        <Button disabled={syncing || issues.length === 0} onClick={onSync} variant="outline">
          <RefreshCw className="h-4 w-4" aria-hidden="true" />
          {t("skills.drift.sync")}
        </Button>
      </div>
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
