import { Cpu, FolderOpen, RefreshCw, RotateCcw, SlidersHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { supportedLocales, type AppLanguage } from "../../i18n/supported-locales";
import { Button } from "../../components/ui/button";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { normalizeDisplayPath } from "../../lib/session-path";
import { useSettings } from "../settings-provider";
import { ucdThemes, type UcdThemeId } from "../../theme/theme-registry";
import { appFontSizes, cliTerminalThemes, type AppFontSize, type AppSettingKey, type AppSettings, type CliTerminalTheme } from "../../types/settings";
import { policyTemplateNames, type PolicyTemplateName } from "../../types/permissions";
import { NetworkProxySection } from "./network-proxy-section";
import { InfoTile, PageHeader, SectionPanel, SettingsDisclosure, SettingsRow } from "./page-parts";
import { FloatingAssistantSettingsSection } from "./floating-assistant-settings-section";
import { DataManagementSection } from "./data-management-section";
import { DirectorySettingField } from "./directory-setting-field";
import { StartupSettingsSection } from "./startup-settings-section";
import { FolderOpenersSection } from "./folder-openers-section";
import { LogManagementSection } from "./log-management-section";

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

function NodeEnvironmentPanel() {
  const { t } = useTranslation();
  const { nodeInfo, refreshNodeInfo } = useSettings();
  return (
    <SectionPanel icon={Cpu} title={t("basic.node")} description={t("basic.nodeDesc")} variant="plain">
      <div className="grid gap-3 lg:grid-cols-[220px_minmax(0,1fr)]">
        <InfoTile icon={Cpu} label={t("basic.nodeVersion")} value={nodeInfo?.version ?? t("basic.nodeUnavailable")} />
        <InfoTile icon={FolderOpen} label={t("basic.nodePath")} value={nodeInfo?.path ? normalizeDisplayPath(nodeInfo.path) : t("basic.nodeUnavailable")} />
        {!nodeInfo?.available ? (
          <div className="rounded border p-3 text-xs ucd-status-warning lg:col-span-2">{nodeInfo?.reason ?? t("basic.nodeUnavailableReason")}</div>
        ) : null}
        <Button className="justify-self-start lg:col-span-2" onClick={() => void refreshNodeInfo()} size="sm" variant="outline">
          <RefreshCw className="h-4 w-4" aria-hidden="true" />
          {t("basic.nodeRefresh")}
        </Button>
      </div>
    </SectionPanel>
  );
}

export function BasicSettingsPage() {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const { error, loading, pickDirectory, reportClientLogEvent, resetSettings, saveSetting, savingKey, settings } = useSettings();
  const busy = loading || savingKey !== null;
  // Only the control being saved locks; the rest of the page stays usable during a save.
  const saving = (key: AppSettingKey) => loading || savingKey === key;
  function select<K extends AppSettingKey>(key: K, value: AppSettings[K]) {
    // The provider already rolls back and surfaces the error; the rejection only needs a home.
    saveSetting(key, value).catch(() => undefined);
  }

  async function saveDefaultFolder(value: string) {
    try {
      await saveSetting("defaultFolderPath", value);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "BasicSettingsPage.saveDefaultFolder",
        details: { requestedDirectory: value },
      });
      throw cause;
    }
  }

  return (
    <div className="mx-auto max-w-[1040px] space-y-5 pb-8">
      {confirmationDialog}
      <PageHeader description={t("basic.description")} icon={SlidersHorizontal} title={t("basic.title")} />

      {error ? <div className="whitespace-pre-line rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {loading ? <div className="rounded-md border border-border p-3 text-sm text-muted-foreground">{t("basic.loading")}</div> : null}

      <div className="grid gap-5">
        <SectionPanel title={t("basic.commonPreferences")} description={t("basic.commonPreferencesDesc")} variant="settings">
          <SettingsRow description={t("basic.languageDesc")} title={t("basic.language")}>
            <SelectField<AppLanguage>
              disabled={saving("applicationLanguage")}
              label={t("basic.language")}
              onChange={(value) => select("applicationLanguage", value)}
              options={supportedLocales.map((locale) => ({
                label: t(locale.labelKey),
                value: locale.id,
              }))}
              value={settings.applicationLanguage}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.themeDesc")} title={t("basic.theme")}>
            <SelectField<UcdThemeId>
              disabled={saving("theme")}
              label={t("basic.theme")}
              onChange={(value) => select("theme", value)}
              options={ucdThemes.map((theme) => ({ label: t(`basic.theme.${theme.id}`), value: theme.id }))}
              value={settings.theme}
            />
          </SettingsRow>
          <SettingsRow
            description={
              <>
                {t("basic.cliTerminalThemeDesc")}
                {/* Only relevant once a light canvas can meet a CLI's own dark-tuned colors. */}
                {settings.cliTerminalTheme === "light" ? <span className="mt-1 block">{t("basic.cliTerminalThemeCompat")}</span> : null}
              </>
            }
            title={t("basic.cliTerminalTheme")}
          >
            <SelectField<CliTerminalTheme>
              disabled={saving("cliTerminalTheme")}
              label={t("basic.cliTerminalTheme")}
              onChange={(value) => select("cliTerminalTheme", value)}
              options={cliTerminalThemes.map((theme) => ({ label: t(`basic.cliTerminalTheme.${theme}`), value: theme }))}
              value={settings.cliTerminalTheme}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.fontSizeDesc")} title={t("basic.fontSize")}>
            <SelectField<AppFontSize>
              disabled={saving("fontSize")}
              label={t("basic.fontSize")}
              onChange={(value) => select("fontSize", value)}
              options={appFontSizes.map((fontSize) => ({ label: `${t(`basic.fontSize.${fontSize}`)} (${fontSize})`, value: fontSize }))}
              value={settings.fontSize}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.defaultPolicyTemplateDesc")} title={t("basic.defaultPolicyTemplate")}>
            <SelectField<PolicyTemplateName>
              disabled={saving("defaultPolicyTemplate")}
              label={t("basic.defaultPolicyTemplate")}
              onChange={(value) => select("defaultPolicyTemplate", value)}
              options={policyTemplateNames.map((template) => ({
                label: t(`settings.agentPolicies.template.${template}`),
                value: template,
              }))}
              value={settings.defaultPolicyTemplate}
            />
          </SettingsRow>
        </SectionPanel>

        <SectionPanel title={t("basic.startupAndWindow")} description={t("basic.startupAndWindowDesc")} variant="settings">
          <StartupSettingsSection />
          <FloatingAssistantSettingsSection />
        </SectionPanel>

        <SectionPanel title={t("basic.workspaceDefaults")} description={t("basic.workspaceDefaultsDesc")} variant="settings">
          <SettingsRow description={t("basic.defaultFolderPathDesc")} title={t("basic.defaultFolderPath")}>
            <DirectorySettingField
              ariaLabel={t("basic.defaultFolderPath")}
              canBrowse={settings.loggingPolicy.canOpenDirectory}
              className="sm:w-[420px]"
              disabled={saving("defaultFolderPath")}
              onPick={pickDirectory}
              onSave={saveDefaultFolder}
              placeholder={t("basic.defaultFolderPathPlaceholder")}
              value={settings.defaultFolderPath}
            />
          </SettingsRow>
          <FolderOpenersSection />
        </SectionPanel>

        <SettingsDisclosure description={t("basic.advancedConfigurationDesc")} storageKey="vanehub.settings.basic.advancedOpen" title={t("basic.advancedConfiguration")}>
          <NetworkProxySection />
          <DataManagementSection />
          <LogManagementSection />
          <NodeEnvironmentPanel />
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
            void confirm({ title: t("basic.resetConfirm"), tone: "danger" })
              .then((confirmed) => { if (confirmed) return resetSettings(); })
              .catch(() => undefined);
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
