import { FolderOpen, ScrollText } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import { useSettings } from "../settings-provider";
import { SectionPanel } from "./page-parts";

function PolicyValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div className="mt-1 break-all text-sm font-medium text-foreground">{value}</div>
    </div>
  );
}

export function LogManagementSection() {
  const { t } = useTranslation();
  const { loading, openLogDirectory, reportClientLogEvent, saveSetting, savingKey, settings } = useSettings();
  const [draft, setDraft] = useState(settings.logDirectory);
  const [error, setError] = useState<string | null>(null);
  const busy = loading || savingKey !== null;

  useEffect(() => {
    setDraft(normalizeDisplayPath(settings.logDirectory));
  }, [settings.logDirectory]);

  function saveDirectory() {
    if (draft === settings.logDirectory) return;
    void saveSetting("logDirectory", draft).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "LogManagementSection.saveDirectory",
        details: { requestedDirectory: draft },
      });
    });
  }

  function openDirectory() {
    setError(null);
    void openLogDirectory().catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "LogManagementSection.openDirectory",
        details: { logDirectory: settings.logDirectory },
      });
    });
  }

  return (
    <SectionPanel icon={ScrollText} title={t("basic.logs")} description={t("basic.logsDesc")} variant="plain">
      <div className="grid gap-4">
        {error ? <div className="rounded border p-3 text-xs ucd-status-danger">{error}</div> : null}
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium text-muted-foreground">{t("basic.logDirectory")}</span>
          <input
            className="ucd-input h-9 w-full rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            disabled={busy}
            onBlur={saveDirectory}
            onChange={(event) => {
              setError(null);
              setDraft(event.target.value);
            }}
            placeholder={t("basic.logDirectoryPlaceholder")}
            value={draft}
          />
        </label>
        <Button className="justify-self-start" disabled={busy || !settings.loggingPolicy.canOpenDirectory} onClick={openDirectory} variant="outline">
          <FolderOpen className="h-4 w-4" aria-hidden="true" />
          {t("basic.openLogDirectory")}
        </Button>
        <dl className="grid gap-3 text-sm sm:grid-cols-2 xl:grid-cols-4">
          <PolicyValue label={t("basic.logRetention")} value={t("basic.logRetentionValue", { days: settings.loggingPolicy.retentionDays })} />
          <PolicyValue label={t("basic.logArchive")} value={settings.loggingPolicy.archiveEnabled ? t("basic.enabled") : t("basic.disabled")} />
          <PolicyValue label={t("basic.logRedaction")} value={settings.loggingPolicy.redactionEnabled ? t("basic.enabled") : t("basic.disabled")} />
          <PolicyValue label={t("basic.logLevels")} value={settings.loggingPolicy.levels.join(" / ")} />
        </dl>
        {!settings.loggingPolicy.canOpenDirectory ? (
          <div className="rounded border p-3 text-xs ucd-status-warning">{t("basic.logOpenUnavailable")}</div>
        ) : null}
      </div>
    </SectionPanel>
  );
}
