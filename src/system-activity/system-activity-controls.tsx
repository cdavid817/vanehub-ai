import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type {
  SystemActivityExportFormat,
  SystemActivityPreferences,
  SystemActivitySession,
} from "../services/system-activity-service";
import { SystemActivityPreferencesPanel } from "./system-activity-preferences";
import { useSystemActivityRebuild } from "./use-system-activity-rebuild";

function defaultPreferences(session: SystemActivitySession): SystemActivityPreferences {
  return {
    scopeKind: session.scopeKind,
    canonicalScopeId: session.canonicalScopeId,
    visible: true,
    minimumTimelineSeverity: "info",
    notificationThreshold: "warning",
    digestCadence: "off",
    readRetentionDays: 180,
    detailRetentionDays: 180,
    exportItemLimit: 1000,
    exportSizeLimitBytes: 10 * 1024 * 1024,
    revision: 0,
  };
}

interface SystemActivityControlsProps {
  session: SystemActivitySession;
  onChanged: () => void;
}

/**
 * Preferences, rebuild, and export controls for one system session. Everything here is
 * projection-side: preference changes and rebuilds never delete authoritative evolution records,
 * and exports leave through the user-selected boundary with a retention disclosure.
 */
export function SystemActivityControls({ session, onChanged }: SystemActivityControlsProps) {
  const { t, i18n } = useTranslation();
  const [message, setMessage] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState("");
  const [exportFormat, setExportFormat] = useState<SystemActivityExportFormat>("json");
  const [preferences, setPreferences] = useState<SystemActivityPreferences | null>(null);

  const report = (text: string) => setMessage(text);
  const rebuild = useSystemActivityRebuild(session, onChanged, report);

  useEffect(() => {
    let cancelled = false;
    agentService
      .getSystemActivityPreferences(session.scopeKind, session.canonicalScopeId)
      .then((stored) => {
        if (!cancelled) setPreferences(stored ?? defaultPreferences(session));
      })
      .catch(() => {
        if (!cancelled) setPreferences(defaultPreferences(session));
      });
    return () => {
      cancelled = true;
    };
  }, [session]);

  const savePreferences = async (next: SystemActivityPreferences) => {
    try {
      const result = await agentService.updateSystemActivityPreferences(next);
      setPreferences(result.preferences);
      report(
        result.outcome === "updated"
          ? t("systemActivity.view.preferencesSaved")
          : t("systemActivity.view.preferencesConflict"),
      );
      onChanged();
    } catch (error) {
      report(error instanceof Error ? error.message : String(error));
    }
  };

  const runExport = async () => {
    try {
      const record = await agentService.exportSystemActivity({
        exportId: `activity-export-${session.sessionId}-${session.lastSequence}`,
        query: { sessionId: session.sessionId },
        format: exportFormat,
        locale: i18n.language,
        targetPath: exportPath,
      });
      report(
        t("systemActivity.view.exportDone", {
          count: record.itemCount,
          complete: record.complete
            ? t("systemActivity.view.exportComplete")
            : t("systemActivity.view.exportPartial"),
        }),
      );
    } catch (error) {
      report(error instanceof Error ? error.message : String(error));
    }
  };

  const chooseExportPath = async () => {
    try {
      const selected = await agentService.chooseSystemActivityExportTarget(exportFormat);
      if (selected) setExportPath(selected);
    } catch (error) {
      report(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <section aria-label={t("systemActivity.view.controls")} className="space-y-3 rounded-lg border border-border p-3">
      {preferences ? (
        <SystemActivityPreferencesPanel
          onSave={(next) => void savePreferences(next)}
          preferences={preferences}
        />
      ) : null}
      <div className="flex items-center gap-2">
        <button
          className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted disabled:opacity-50"
          data-testid="system-activity-rebuild"
          disabled={rebuild.progress !== null}
          onClick={() => void rebuild.run()}
          type="button"
        >
          {t("systemActivity.view.rebuild")}
        </button>
        {rebuild.progress ? (
          <button
            className="rounded-md border border-destructive/50 px-2 py-1 text-xs text-destructive hover:bg-destructive/10 disabled:opacity-50"
            data-testid="system-activity-rebuild-cancel"
            disabled={rebuild.progress.phase === "cancelling"}
            onClick={() => void rebuild.cancel()}
            type="button"
          >
            {t("systemActivity.view.rebuildCancel")}
          </button>
        ) : null}
      </div>
      {rebuild.progress ? (
        <div className="space-y-1" data-testid="system-activity-rebuild-progress">
          <div className="flex justify-between gap-2 text-[11px] text-muted-foreground">
            <span>{t(`systemActivity.view.rebuildPhase.${rebuild.progress.phase}`)}</span>
            <span className="font-mono">
              {rebuild.progress.processedItems}/{rebuild.progress.itemBudget}
            </span>
          </div>
          <progress
            aria-label={t("systemActivity.view.rebuildProgress")}
            className="h-1.5 w-full accent-primary"
            max={rebuild.progress.itemBudget}
            value={rebuild.progress.processedItems}
          />
        </div>
      ) : null}
      <div className="space-y-2">
        <label className="block text-xs text-muted-foreground" htmlFor="system-activity-export-path">
          {t("systemActivity.view.exportPath")}
        </label>
        <div className="flex gap-2">
          <input
            className="min-w-0 flex-1 rounded-md border border-border bg-muted px-2 py-1 text-xs"
            id="system-activity-export-path"
            readOnly
            value={exportPath}
          />
          <button
            className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
            onClick={() => void chooseExportPath()}
            type="button"
          >
            {t("systemActivity.view.exportChoose")}
          </button>
        </div>
        <div className="flex items-center gap-2">
          <select
            aria-label={t("systemActivity.view.exportFormat")}
            className="rounded-md border border-border bg-background px-2 py-1 text-xs"
            onChange={(event) => setExportFormat(event.target.value as SystemActivityExportFormat)}
            value={exportFormat}
          >
            <option value="json">JSON</option>
            <option value="markdown">Markdown</option>
          </select>
          <button
            className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted disabled:opacity-50"
            data-testid="system-activity-export"
            disabled={exportPath.trim() === ""}
            onClick={() => void runExport()}
            type="button"
          >
            {t("systemActivity.view.export")}
          </button>
        </div>
        <p className="text-[11px] text-muted-foreground">{t("systemActivity.view.exportDisclosure")}</p>
      </div>
      {message ? (
        <p className="text-xs text-muted-foreground" data-testid="system-activity-controls-message" role="status">
          {message}
        </p>
      ) : null}
    </section>
  );
}
