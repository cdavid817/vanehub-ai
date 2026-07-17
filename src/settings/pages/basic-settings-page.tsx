import { FolderOpen, RotateCcw, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { useSettings } from "../settings-provider";
import { ucdThemes } from "../../theme/theme-registry";
import { appFontSizes, appLanguages, type AppFontSize, type AppLanguage } from "../../types/settings";
import { NetworkProxySection } from "./network-proxy-section";
import { PageHeader, SectionPanel } from "./page-parts";
import { FloatingAssistantSettingsSection } from "./floating-assistant-settings-section";

function SelectField<T extends string>({
  disabled,
  label,
  onChange,
  options,
  value,
}: {
  disabled?: boolean;
  label: string;
  onChange: (value: T) => void;
  options: Array<{ label: string; value: T }>;
  value: T;
}) {
  return (
    <label className="grid gap-1 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <select
        className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
        disabled={disabled}
        onChange={(event) => onChange(event.target.value as T)}
        value={value}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

export function BasicSettingsPage() {
  const { t } = useTranslation();
  const { error, loading, nodeInfo, openLogDirectory, reportClientLogEvent, resetSettings, saveSetting, savingKey, settings } = useSettings();
  const [folderDraft, setFolderDraft] = useState(settings.defaultFolderPath);
  const [logDirectoryDraft, setLogDirectoryDraft] = useState(settings.logDirectory);
  const [logError, setLogError] = useState<string | null>(null);
  const busy = loading || savingKey !== null;

  useEffect(() => {
    setFolderDraft(settings.defaultFolderPath);
  }, [settings.defaultFolderPath]);

  useEffect(() => {
    setLogDirectoryDraft(settings.logDirectory);
  }, [settings.logDirectory]);

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button disabled={busy} onClick={() => void resetSettings()} variant="outline">
            <RotateCcw className="h-4 w-4" aria-hidden="true" />
            {t("basic.reset")}
          </Button>
        }
        description={t("basic.description")}
        icon={Settings}
        title={t("basic.title")}
      />

      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {loading ? <div className="rounded-md border border-border p-3 text-sm text-muted-foreground">{t("basic.loading")}</div> : null}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid gap-4">
          <SectionPanel title={t("basic.appSettings")} description={t("basic.appSettingsDesc")}>
            <div className="grid gap-4 md:grid-cols-2">
              <SelectField<AppLanguage>
                disabled={busy}
                label={t("basic.language")}
                onChange={(value) => void saveSetting("applicationLanguage", value)}
                options={appLanguages.map((language) => ({
                  label: language === "zh-CN" ? t("basic.language.zh") : t("basic.language.en"),
                  value: language,
                }))}
                value={settings.applicationLanguage}
              />
              <SelectField<AppFontSize>
                disabled={busy}
                label={t("basic.fontSize")}
                onChange={(value) => void saveSetting("fontSize", value)}
                options={appFontSizes.map((fontSize) => ({ label: fontSize, value: fontSize }))}
                value={settings.fontSize}
              />
              <SelectField
                disabled={busy}
                label={t("basic.theme")}
                onChange={(value) => void saveSetting("theme", value)}
                options={ucdThemes.map((theme) => ({
                  label: theme.id === "futuristic" ? t("basic.theme.futuristic") : t("basic.theme.minimal"),
                  value: theme.id,
                }))}
                value={settings.theme}
              />
              <label className="grid gap-1 text-sm">
                <span className="text-muted-foreground">{t("basic.defaultFolder")}</span>
                <input
                  className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  disabled={busy}
                  onBlur={() => {
                    if (folderDraft !== settings.defaultFolderPath) {
                      void saveSetting("defaultFolderPath", folderDraft);
                    }
                  }}
                  onChange={(event) => setFolderDraft(event.target.value)}
                  placeholder={t("basic.defaultFolderPlaceholder")}
                  value={folderDraft}
                />
              </label>
            </div>
          </SectionPanel>

          <FloatingAssistantSettingsSection />

          <NetworkProxySection />

          <SectionPanel title={t("basic.logs")} description={t("basic.logsDesc")}>
            <div className="grid gap-4">
              {logError ? <div className="rounded border p-3 text-xs ucd-status-danger">{logError}</div> : null}
              <label className="grid gap-1 text-sm">
                <span className="text-muted-foreground">{t("basic.logDirectory")}</span>
                <input
                  className="ucd-input h-9 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  disabled={busy}
                  onBlur={() => {
                    if (logDirectoryDraft !== settings.logDirectory) {
                      void saveSetting("logDirectory", logDirectoryDraft).catch((err) => {
                        const message = err instanceof Error ? err.message : String(err);
                        setLogError(message);
                        void reportClientLogEvent({
                          level: "error",
                          kind: "critical-operation-failure",
                          message,
                          source: "BasicSettingsPage.saveLogDirectory",
                          details: { requestedDirectory: logDirectoryDraft },
                        });
                      });
                    }
                  }}
                  onChange={(event) => {
                    setLogError(null);
                    setLogDirectoryDraft(event.target.value);
                  }}
                  placeholder={t("basic.logDirectoryPlaceholder")}
                  value={logDirectoryDraft}
                />
              </label>
              <Button
                disabled={busy || !settings.loggingPolicy.canOpenDirectory}
                onClick={() => {
                  setLogError(null);
                  void openLogDirectory().catch((err) => {
                    const message = err instanceof Error ? err.message : String(err);
                    setLogError(message);
                    void reportClientLogEvent({
                      level: "error",
                      kind: "critical-operation-failure",
                      message,
                      source: "BasicSettingsPage.openLogDirectory",
                      details: { logDirectory: settings.logDirectory },
                    });
                  });
                }}
                variant="outline"
              >
                <FolderOpen className="h-4 w-4" aria-hidden="true" />
                {t("basic.openLogDirectory")}
              </Button>
              <dl className="grid gap-2 text-sm text-muted-foreground md:grid-cols-2">
                <div>
                  <dt>{t("basic.logRetention")}</dt>
                  <dd className="font-medium text-foreground">{t("basic.logRetentionValue", { days: settings.loggingPolicy.retentionDays })}</dd>
                </div>
                <div>
                  <dt>{t("basic.logArchive")}</dt>
                  <dd className="font-medium text-foreground">{settings.loggingPolicy.archiveEnabled ? t("basic.enabled") : t("basic.disabled")}</dd>
                </div>
                <div>
                  <dt>{t("basic.logRedaction")}</dt>
                  <dd className="font-medium text-foreground">{settings.loggingPolicy.redactionEnabled ? t("basic.enabled") : t("basic.disabled")}</dd>
                </div>
                <div>
                  <dt>{t("basic.logLevels")}</dt>
                  <dd className="font-medium text-foreground">{settings.loggingPolicy.levels.join(" / ")}</dd>
                </div>
              </dl>
              {!settings.loggingPolicy.canOpenDirectory ? (
                <div className="rounded border p-3 text-xs ucd-status-warning">{t("basic.logOpenUnavailable")}</div>
              ) : null}
            </div>
          </SectionPanel>
        </div>

        <div className="grid gap-4">
          <SectionPanel title={t("basic.node")} description={t("basic.nodeDesc")}>
            <div className="grid gap-4 text-sm">
              <div>
                <div className="text-muted-foreground">{t("basic.nodePath")}</div>
                <div className="mt-1 break-all font-medium">
                  {nodeInfo?.path ?? t("basic.nodeUnavailable")}
                </div>
              </div>
              <div>
                <div className="text-muted-foreground">{t("basic.nodeVersion")}</div>
                <div className="mt-1 font-medium">{nodeInfo?.version ?? t("basic.nodeUnavailable")}</div>
              </div>
              {!nodeInfo?.available ? (
                <div className="rounded border p-3 text-xs ucd-status-warning">
                  {nodeInfo?.reason ?? t("basic.nodeUnavailableReason")}
                </div>
              ) : null}
            </div>
          </SectionPanel>

          <SectionPanel title={t("basic.storage")} description={t("basic.storageDesc")}>
            <ul className="grid gap-2 text-sm text-muted-foreground">
              <li>{t("basic.desktopStorage")}</li>
              <li>{t("basic.webStorage")}</li>
              <li>{t("basic.themeEntry")}</li>
            </ul>
          </SectionPanel>
        </div>
      </div>
    </div>
  );
}
