import { Cpu, FolderOpen, RotateCcw, ScrollText, Settings } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import { useSettings } from "../settings-provider";
import { ucdThemes } from "../../theme/theme-registry";
import { appFontSizes, appLanguages, type AppFontSize, type AppLanguage } from "../../types/settings";
import { NetworkProxySection } from "./network-proxy-section";
import { PageHeader, SectionPanel } from "./page-parts";
import { FloatingAssistantSettingsSection } from "./floating-assistant-settings-section";
import { DataManagementSection } from "./data-management-section";
import { StartupSettingsSection } from "./startup-settings-section";
import { FolderOpenersSection } from "./folder-openers-section";

function InfoBlock({ icon: Icon, label, value }: { icon?: LucideIcon; label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3">
      <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
        {Icon ? <Icon className="h-3.5 w-3.5 text-primary" aria-hidden="true" /> : null}
        {label}
      </div>
      <div className="mt-1 break-all text-sm font-medium text-foreground">{value}</div>
    </div>
  );
}

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
    <label className="grid gap-1.5 text-sm">
      <span className="font-medium text-muted-foreground">{label}</span>
      <select
        className="ucd-input h-9 w-full rounded px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
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

function NodeEnvironmentPanel({
  nodeInfo,
  t,
}: {
  nodeInfo: ReturnType<typeof useSettings>["nodeInfo"];
  t: ReturnType<typeof useTranslation>["t"];
}) {
  return (
    <SectionPanel icon={Cpu} title={t("basic.node")} description={t("basic.nodeDesc")}>
      <div className="grid gap-3 lg:grid-cols-[220px_minmax(0,1fr)]">
        <InfoBlock icon={Cpu} label={t("basic.nodeVersion")} value={nodeInfo?.version ?? t("basic.nodeUnavailable")} />
        <InfoBlock icon={FolderOpen} label={t("basic.nodePath")} value={nodeInfo?.path ? normalizeDisplayPath(nodeInfo.path) : t("basic.nodeUnavailable")} />
        {!nodeInfo?.available ? (
          <div className="rounded border p-3 text-xs ucd-status-warning lg:col-span-2">{nodeInfo?.reason ?? t("basic.nodeUnavailableReason")}</div>
        ) : null}
      </div>
    </SectionPanel>
  );
}

export function BasicSettingsPage() {
  const { t } = useTranslation();
  const { error, loading, nodeInfo, openLogDirectory, reportClientLogEvent, resetSettings, saveSetting, savingKey, settings } = useSettings();
  const [logDirectoryDraft, setLogDirectoryDraft] = useState(settings.logDirectory);
  const [logError, setLogError] = useState<string | null>(null);
  const busy = loading || savingKey !== null;

  useEffect(() => {
    setLogDirectoryDraft(normalizeDisplayPath(settings.logDirectory));
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

      <div className="grid gap-4">
        <SectionPanel icon={Settings} title={t("basic.appSettings")} description={t("basic.appSettingsDesc")}>
          <div className="grid gap-4 md:grid-cols-3">
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
            <SelectField<AppFontSize>
              disabled={busy}
              label={t("basic.fontSize")}
              onChange={(value) => void saveSetting("fontSize", value)}
              options={appFontSizes.map((fontSize) => ({ label: fontSize, value: fontSize }))}
              value={settings.fontSize}
            />
          </div>
        </SectionPanel>

        <StartupSettingsSection />

        <FolderOpenersSection />

        <SectionPanel icon={ScrollText} title={t("basic.logs")} description={t("basic.logsDesc")}>
          <div className="grid gap-4">
            {logError ? <div className="rounded border p-3 text-xs ucd-status-danger">{logError}</div> : null}
            <label className="grid gap-1.5 text-sm">
              <span className="font-medium text-muted-foreground">{t("basic.logDirectory")}</span>
              <input
                className="ucd-input h-9 w-full rounded px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
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
            <div className="flex flex-wrap items-center gap-2">
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
            </div>
            <dl className="grid gap-3 text-sm sm:grid-cols-2 xl:grid-cols-4">
              <InfoBlock label={t("basic.logRetention")} value={t("basic.logRetentionValue", { days: settings.loggingPolicy.retentionDays })} />
              <InfoBlock label={t("basic.logArchive")} value={settings.loggingPolicy.archiveEnabled ? t("basic.enabled") : t("basic.disabled")} />
              <InfoBlock label={t("basic.logRedaction")} value={settings.loggingPolicy.redactionEnabled ? t("basic.enabled") : t("basic.disabled")} />
              <InfoBlock label={t("basic.logLevels")} value={settings.loggingPolicy.levels.join(" / ")} />
            </dl>
            {!settings.loggingPolicy.canOpenDirectory ? (
              <div className="rounded border p-3 text-xs ucd-status-warning">{t("basic.logOpenUnavailable")}</div>
            ) : null}
          </div>
        </SectionPanel>

        <DataManagementSection />

        <NodeEnvironmentPanel nodeInfo={nodeInfo} t={t} />

        <NetworkProxySection />
      </div>

      <FloatingAssistantSettingsSection />
    </div>
  );
}
