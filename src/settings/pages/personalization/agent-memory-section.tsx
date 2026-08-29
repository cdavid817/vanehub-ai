import { useTranslation } from "react-i18next";
import { useSettings } from "../../settings-provider";
import { SectionPanel, SettingsRow } from "../page-parts";

function MemoryToggle({ ariaLabel, checked, disabled, onToggle }: { ariaLabel: string; checked: boolean; disabled: boolean; onToggle: () => void }) {
  return (
    <button
      aria-checked={checked}
      aria-label={ariaLabel}
      className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${checked ? "bg-primary" : "bg-muted-foreground/40"} disabled:opacity-40`}
      disabled={disabled}
      onClick={onToggle}
      role="switch"
      type="button"
    >
      <span className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-background shadow-sm transition-transform ${checked ? "translate-x-5" : "translate-x-0"}`} />
    </button>
  );
}

/**
 * The memory policy toggles.
 *
 * No service prop: these are application settings, which the settings context owns. The store's
 * contents live in the list panel beside this one, and keeping the two apart is what stopped this
 * component reading every memory in full to render a page it no longer draws.
 */
export function AgentMemorySection() {
  const { t } = useTranslation();
  const { loading, reportClientLogEvent, saveSetting, savingKey, settings } = useSettings();
  const settingsBusy = loading || savingKey !== null;
  const memoryEnabled = settings.memoryEnabled;

  function toggleSetting(key: "memoryEnabled" | "memoryToolAssistedChatsEnabled", nextValue: boolean) {
    void saveSetting(key, nextValue).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "AgentMemorySection.toggleSetting",
        details: { key },
      });
    });
  }

  return (
    <SectionPanel description={t("personalization.memory.description")} title={t("personalization.memory.title")} variant="settings">
      <SettingsRow description={t("personalization.memory.enabledDesc")} title={t("personalization.memory.enabled")}>
        <MemoryToggle
          ariaLabel={t("personalization.memory.enabled")}
          checked={memoryEnabled}
          disabled={settingsBusy}
          onToggle={() => toggleSetting("memoryEnabled", !memoryEnabled)}
        />
      </SettingsRow>
      <SettingsRow description={t("personalization.memory.toolAssistedDesc")} title={t("personalization.memory.toolAssisted")}>
        <MemoryToggle
          ariaLabel={t("personalization.memory.toolAssisted")}
          checked={settings.memoryToolAssistedChatsEnabled}
          disabled={settingsBusy || !memoryEnabled}
          onToggle={() => toggleSetting("memoryToolAssistedChatsEnabled", !settings.memoryToolAssistedChatsEnabled)}
        />
      </SettingsRow>
    </SectionPanel>
  );
}
