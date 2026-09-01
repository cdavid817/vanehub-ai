import { Cpu, FolderOpen, RotateCcw } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { supportedLocales, type AppLanguage } from "../../i18n/supported-locales";
import { Button } from "../../components/ui/button";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { normalizeDisplayPath } from "../../lib/session-path";
import { useSettings } from "../settings-provider";
import { pickPageStatus } from "../settings-page-status";
import type { SettingsPageStatus } from "../settings-page-types";
import { ucdThemes } from "../../theme/theme-registry";
import { appFontSizes, type AppFontSize } from "../../types/settings";
import type { AppSettingKey, AppSettings } from "../../types/settings";
import { policyTemplateNames, type PolicyTemplateName } from "../../types/permissions";
import type { MutationState } from "../../ui/async/mutation-state";
import { NetworkProxySection } from "./network-proxy-section";
import { DangerZone, SectionPanel, SettingsDisclosure, SettingsRow } from "./page-parts";
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
    <SectionPanel icon={Cpu} title={t("basic.node")} description={t("basic.nodeDesc")} variant="plain">
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

export function BasicSettingsPage({ onStatusChange }: { onStatusChange?: (status: SettingsPageStatus | null) => void }) {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const { error, errorKey, loading, nodeInfo, reportClientLogEvent, resetSettings, saveSetting, savingKey, settings } = useSettings();
  const [defaultFolderDraft, setDefaultFolderDraft] = useState(settings.defaultFolderPath);
  const [defaultFolderError, setDefaultFolderError] = useState<string | null>(null);
  const busy = loading || savingKey !== null;

  // Task 12.16: the same two conditions already rendered below (the error banner, and
  // NodeEnvironmentPanel's own unavailable banner) -- reported so this page's nav entry can flag
  // either one while the user is looking elsewhere.
  useEffect(() => {
    onStatusChange?.(pickPageStatus([
      error ? { kind: "error", labelKey: "basic.status.error" } : null,
      !nodeInfo?.available ? { kind: "dependency-unavailable", labelKey: "basic.status.nodeUnavailable" } : null,
    ]));
    return () => onStatusChange?.(null);
  }, [error, nodeInfo?.available, onStatusChange]);

  /** Task 12.10: per-row pending/error, derived from the provider's own single-in-flight
   *  `savingKey`/`errorKey` rather than a page-wide busy flag or one global banner. No retry
   *  action -- the failed value already rolled back in the `<select>` itself, so re-selecting it
   *  *is* the retry; there is no separate cached value worth replaying. */
  function rowMutation(key: AppSettingKey): MutationState | undefined {
    if (savingKey === key) return { targetKey: key, pending: true };
    if (errorKey === key && error) return { targetKey: key, pending: false, error: { kind: "error", message: error, retryable: false } };
    return undefined;
  }

  /** `saveSetting` re-throws after its own internal handling (state rollback, `error`/`errorKey`)
   *  so a caller that wants additional handling can still get it -- these rows do not, so the
   *  bare `void saveSetting(...)` this replaces left the rejection genuinely unhandled at the call
   *  site. The provider already did everything these rows need; this only acknowledges that. */
  function saveField<K extends AppSettingKey>(key: K, value: AppSettings[K]) {
    void saveSetting(key, value).catch(() => undefined);
  }

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
      {confirmationDialog}
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
          <SettingsRow description={t("basic.languageDesc")} mutation={rowMutation("applicationLanguage")} title={t("basic.language")}>
            <SelectField<AppLanguage>
              disabled={busy}
              label={t("basic.language")}
              onChange={(value) => saveField("applicationLanguage", value)}
              options={supportedLocales.map((locale) => ({
                label: t(locale.labelKey),
                value: locale.id,
              }))}
              value={settings.applicationLanguage}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.themeDesc")} mutation={rowMutation("theme")} title={t("basic.theme")}>
            <SelectField
              disabled={busy}
              label={t("basic.theme")}
              onChange={(value) => saveField("theme", value)}
              options={ucdThemes.map((theme) => ({
                label: theme.id === "futuristic" ? t("basic.theme.futuristic") : t("basic.theme.minimal"),
                value: theme.id,
              }))}
              value={settings.theme}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.fontSizeDesc")} mutation={rowMutation("fontSize")} title={t("basic.fontSize")}>
            <SelectField<AppFontSize>
              disabled={busy}
              label={t("basic.fontSize")}
              onChange={(value) => saveField("fontSize", value)}
              options={appFontSizes.map((fontSize) => ({ label: fontSize, value: fontSize }))}
              value={settings.fontSize}
            />
          </SettingsRow>
          <SettingsRow description={t("basic.defaultPolicyTemplateDesc")} mutation={rowMutation("defaultPolicyTemplate")} title={t("basic.defaultPolicyTemplate")}>
            <SelectField<PolicyTemplateName>
              disabled={busy}
              label={t("basic.defaultPolicyTemplate")}
              onChange={(value) => saveField("defaultPolicyTemplate", value)}
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

      <DangerZone description={t("basic.resetDescription")} title={t("basic.resetSection")}>
        <Button
          className="shrink-0"
          disabled={busy}
          onClick={() => {
            void confirm({ title: t("basic.resetConfirm"), tone: "danger" })
              .then((confirmed) => { if (confirmed) void resetSettings(); });
          }}
          variant="destructive"
        >
          <RotateCcw className="h-4 w-4" aria-hidden="true" />
          {t("basic.reset")}
        </Button>
      </DangerZone>
    </div>
  );
}
