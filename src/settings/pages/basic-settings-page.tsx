import { Cpu, FolderOpen, RotateCcw } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { supportedLocales, type AppLanguage } from "../../i18n/supported-locales";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import { useSettings } from "../settings-provider";
import { ucdThemes } from "../../theme/theme-registry";
import { appFontSizes, type AppFontSize } from "../../types/settings";
import { NetworkProxySection } from "./network-proxy-section";
import { SectionPanel, SettingsDisclosure, SettingsRow } from "./page-parts";
import { FloatingAssistantSettingsSection } from "./floating-assistant-settings-section";
import { DataManagementSection } from "./data-management-section";
import { StartupSettingsSection } from "./startup-settings-section";
import { FolderOpenersSection } from "./folder-openers-section";
import { LogManagementSection } from "./log-management-section";

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
    <label className="block text-sm">
      <span className="sr-only">{label}</span>
      <select
        aria-label={label}
        className="ucd-input h-9 w-full min-w-40 rounded-lg px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring sm:w-auto"
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
    <SectionPanel icon={Cpu} title={t("basic.node")} description={t("basic.nodeDesc")} variant="settings">
      <div className="grid gap-3 p-5 sm:p-6 lg:grid-cols-[220px_minmax(0,1fr)]">
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
  const { error, loading, nodeInfo, reportClientLogEvent, resetSettings, saveSetting, savingKey, settings } = useSettings();
  const [defaultFolderDraft, setDefaultFolderDraft] = useState(settings.defaultFolderPath);
  const [defaultFolderError, setDefaultFolderError] = useState<string | null>(null);
  const busy = loading || savingKey !== null;

  useEffect(() => {
    setDefaultFolderDraft(normalizeDisplayPath(settings.defaultFolderPath));
  }, [settings.defaultFolderPath]);

  function saveDefaultFolder() {
    if (defaultFolderDraft === settings.defaultFolderPath) return;
    void saveSetting("defaultFolderPath", defaultFolderDraft).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      setDefaultFolderError(message);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "BasicSettingsPage.saveDefaultFolder",
        details: { requestedDirectory: defaultFolderDraft },
      });
    });
  }

  return (
    <div className="mx-auto max-w-[1040px] space-y-5 pb-8">
      <header className="border-b border-border pb-5">
        <div>
          <div className="mb-1 text-xs font-medium text-muted-foreground">{t("app.settings.breadcrumb")}</div>
          <h2 className="text-xl font-semibold leading-tight tracking-tight">{t("basic.title")}</h2>
          <p className="mt-1.5 text-sm leading-6 text-muted-foreground">{t("basic.description")}</p>
        </div>
      </header>

      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {loading ? <div className="rounded-md border border-border p-3 text-sm text-muted-foreground">{t("basic.loading")}</div> : null}

      <div className="grid gap-5">
        <SectionPanel title={t("basic.commonPreferences")} description={t("basic.commonPreferencesDesc")} variant="settings">
          <SettingsRow description={t("basic.languageDesc")} title={t("basic.language")}>
            <SelectField<AppLanguage>
              disabled={busy}
              label={t("basic.language")}
              onChange={(value) => void saveSetting("applicationLanguage", value)}
              options={supportedLocales.map((locale) => ({
                label: t(locale.labelKey),
                value: locale.id,
              }))}
              value={settings.applicationLanguage}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.themeDesc")} title={t("basic.theme")}>
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
          </SettingsRow>
          <SettingsRow description={t("basic.fontSizeDesc")} title={t("basic.fontSize")}>
            <SelectField<AppFontSize>
              disabled={busy}
              label={t("basic.fontSize")}
              onChange={(value) => void saveSetting("fontSize", value)}
              options={appFontSizes.map((fontSize) => ({ label: fontSize, value: fontSize }))}
              value={settings.fontSize}
            />
          </SettingsRow>
        </SectionPanel>

        <SectionPanel title={t("basic.startupAndWindow")} description={t("basic.startupAndWindowDesc")} variant="settings">
          <StartupSettingsSection />
          <FloatingAssistantSettingsSection />
        </SectionPanel>

        <SectionPanel title={t("basic.workspaceDefaults")} description={t("basic.workspaceDefaultsDesc")} variant="settings">
          <SettingsRow description={t("basic.defaultFolderPathDesc")} title={t("basic.defaultFolderPath")}>
            <input
              aria-label={t("basic.defaultFolderPath")}
              className="ucd-input h-9 w-full rounded-lg px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring sm:w-[360px]"
              disabled={busy}
              onBlur={saveDefaultFolder}
              onChange={(event) => {
                setDefaultFolderError(null);
                setDefaultFolderDraft(event.target.value);
              }}
              placeholder={t("basic.defaultFolderPathPlaceholder")}
              value={defaultFolderDraft}
            />
          </SettingsRow>
          {defaultFolderError ? <div className="border-b border-border/70 px-5 py-3 text-xs ucd-status-danger sm:px-6">{defaultFolderError}</div> : null}
          <FolderOpenersSection />
        </SectionPanel>

        <SettingsDisclosure title={t("basic.advancedConfiguration")} description={t("basic.advancedConfigurationDesc")}>
          <NetworkProxySection />
          <DataManagementSection />
          <LogManagementSection />
          <NodeEnvironmentPanel nodeInfo={nodeInfo} t={t} />
        </SettingsDisclosure>
      </div>

      <footer className="flex flex-col gap-3 border-t border-border pt-5 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <div className="text-sm font-medium text-foreground">{t("basic.resetSection")}</div>
          <div className="mt-0.5 text-xs leading-5 text-muted-foreground">{t("basic.resetDescription")}</div>
        </div>
        <Button
          className="shrink-0"
          disabled={busy}
          onClick={() => {
            if (window.confirm(t("basic.resetConfirm"))) void resetSettings();
          }}
          variant="outline"
        >
          <RotateCcw className="h-4 w-4" aria-hidden="true" />
          {t("basic.reset")}
        </Button>
      </footer>
    </div>
  );
}
