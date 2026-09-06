import { FolderOpen, ScrollText } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { useSettings } from "../settings-provider";
import { DirectorySettingField } from "./directory-setting-field";
import { InfoTile, SectionPanel } from "./page-parts";

export function LogManagementSection() {
  const { t } = useTranslation();
  const { loading, openLogDirectory, pickDirectory, reportClientLogEvent, saveSetting, savingKey, settings } = useSettings();
  const [error, setError] = useState<string | null>(null);
  const busy = loading || savingKey === "logDirectory";
  const nativeAvailable = settings.loggingPolicy.canOpenDirectory;

  async function saveDirectory(value: string) {
    try {
      await saveSetting("logDirectory", value);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "LogManagementSection.saveDirectory",
        details: { requestedDirectory: value },
      });
      throw cause;
    }
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
        <div className="grid gap-1.5 text-sm">
          <span className="font-medium text-muted-foreground">{t("basic.logDirectory")}</span>
          <DirectorySettingField
            ariaLabel={t("basic.logDirectory")}
            canBrowse={nativeAvailable}
            disabled={busy}
            onPick={pickDirectory}
            onSave={saveDirectory}
            placeholder={t("basic.logDirectoryPlaceholder")}
            value={settings.logDirectory}
          />
        </div>
        <Button className="justify-self-start" disabled={busy || !nativeAvailable} onClick={openDirectory} variant="outline">
          <FolderOpen className="h-4 w-4" aria-hidden="true" />
          {t("basic.openLogDirectory")}
        </Button>
        <dl className="grid gap-3 text-sm sm:grid-cols-2 xl:grid-cols-4">
          <InfoTile label={t("basic.logRetention")} value={t("basic.logRetentionValue", { days: settings.loggingPolicy.retentionDays })} />
          <InfoTile label={t("basic.logArchive")} value={settings.loggingPolicy.archiveEnabled ? t("basic.enabled") : t("basic.disabled")} />
          <InfoTile label={t("basic.logRedaction")} value={settings.loggingPolicy.redactionEnabled ? t("basic.enabled") : t("basic.disabled")} />
          <InfoTile label={t("basic.logLevels")} value={settings.loggingPolicy.levels.join(" / ")} />
        </dl>
        {!nativeAvailable ? (
          <div className="rounded border p-3 text-xs ucd-status-warning">{t("basic.logOpenUnavailable")}</div>
        ) : null}
      </div>
    </SectionPanel>
  );
}
