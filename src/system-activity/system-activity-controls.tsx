import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type {
  ActivityDigestCadence,
  SystemActivityExportFormat,
  SystemActivityPreferences,
  SystemActivitySession,
} from "../services/system-activity-service";

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

  const runRebuild = async () => {
    try {
      const rebuild = await agentService.beginSystemActivityRebuild(
        session.scopeKind,
        session.canonicalScopeId,
        10_000,
      );
      let step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
      while (step.step === "running") {
        step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
      }
      await agentService.validateSystemActivityRebuild(rebuild.rebuildId);
      let activation = await agentService.activateSystemActivityRebuild(rebuild.rebuildId);
      while (activation.step === "needsCatchUp") {
        step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
        while (step.step === "running") {
          step = await agentService.advanceSystemActivityRebuild(rebuild.rebuildId, 100);
        }
        await agentService.validateSystemActivityRebuild(rebuild.rebuildId);
        activation = await agentService.activateSystemActivityRebuild(rebuild.rebuildId);
      }
      report(t("systemActivity.view.rebuildDone"));
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

  return (
    <section aria-label={t("systemActivity.view.controls")} className="space-y-3 rounded-lg border border-border p-3">
      {preferences ? (
        <div className="space-y-2 text-xs" data-testid="system-activity-preferences">
          <label className="flex items-center gap-2">
            <input
              checked={preferences.visible}
              onChange={(event) => void savePreferences({ ...preferences, visible: event.target.checked })}
              type="checkbox"
            />
            {t("systemActivity.view.preferenceVisible")}
          </label>
          <label className="flex items-center justify-between gap-2">
            {t("systemActivity.view.preferenceDigest")}
            <select
              className="rounded-md border border-border bg-background px-1 py-0.5"
              onChange={(event) => void savePreferences({ ...preferences, digestCadence: event.target.value as ActivityDigestCadence })}
              value={preferences.digestCadence}
            >
              <option value="off">{t("systemActivity.view.digestOff")}</option>
              <option value="hourly">{t("systemActivity.view.digestHourly")}</option>
              <option value="daily">{t("systemActivity.view.digestDaily")}</option>
            </select>
          </label>
          <label className="flex items-center justify-between gap-2">
            {t("systemActivity.view.preferenceRetention")}
            <input
              className="w-16 rounded-md border border-border bg-background px-1 py-0.5"
              max={365}
              min={30}
              onChange={(event) => {
                const days = Number.parseInt(event.target.value, 10);
                if (Number.isNaN(days) || days < 30 || days > 365) return;
                void savePreferences({ ...preferences, detailRetentionDays: days });
              }}
              type="number"
              value={preferences.detailRetentionDays}
            />
          </label>
        </div>
      ) : null}
      <div className="flex items-center gap-2">
        <button
          className="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
          data-testid="system-activity-rebuild"
          onClick={() => void runRebuild()}
          type="button"
        >
          {t("systemActivity.view.rebuild")}
        </button>
      </div>
      <div className="space-y-2">
        <label className="block text-xs text-muted-foreground" htmlFor="system-activity-export-path">
          {t("systemActivity.view.exportPath")}
        </label>
        <input
          className="w-full rounded-md border border-border bg-background px-2 py-1 text-xs"
          id="system-activity-export-path"
          onChange={(event) => setExportPath(event.target.value)}
          value={exportPath}
        />
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
