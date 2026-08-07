import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../settings-provider";
import { countCustomInstructionsCharacters, customInstructionsFieldCharacterLimit } from "../../../types/settings";
import { SectionPanel, SettingsRow } from "../page-parts";

function CustomInstructionsField({
  busy,
  label,
  onCommit,
  placeholder,
  value,
}: {
  busy: boolean;
  label: string;
  onCommit: (value: string) => void;
  placeholder: string;
  value: string;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(value);
  const draftLength = countCustomInstructionsCharacters(draft);
  const overLimit = draftLength > customInstructionsFieldCharacterLimit;

  useEffect(() => {
    setDraft(value);
  }, [value]);

  return (
    <div className="px-5 py-4 sm:px-6">
      <div className="mb-2 flex items-center justify-between gap-3">
        <span className="text-sm font-medium leading-5 text-foreground">{label}</span>
        <span className={`text-xs ${overLimit ? "ucd-status-danger" : "text-muted-foreground"}`}>
          {t("personalization.customInstructions.characterCount", {
            count: draftLength,
            limit: customInstructionsFieldCharacterLimit,
          })}
        </span>
      </div>
      <textarea
        aria-label={label}
        className="ucd-input min-h-28 w-full rounded-lg px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        disabled={busy}
        onBlur={() => {
          if (!overLimit && draft !== value) onCommit(draft);
        }}
        onChange={(event) => setDraft(event.target.value)}
        placeholder={placeholder}
        value={draft}
      />
    </div>
  );
}

export function CustomInstructionsSection() {
  const { t } = useTranslation();
  const { loading, reportClientLogEvent, saveSetting, savingKey, settings } = useSettings();
  const busy = loading || savingKey !== null;
  const enabled = settings.customInstructionsEnabled;

  function commit<K extends "customInstructionsAboutUser" | "customInstructionsStyleRules">(key: K, value: string) {
    void saveSetting(key, value).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "CustomInstructionsSection.commit",
        details: { key },
      });
    });
  }

  function toggleEnabled() {
    void saveSetting("customInstructionsEnabled", !enabled).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "CustomInstructionsSection.toggleEnabled",
      });
    });
  }

  return (
    <SectionPanel description={t("personalization.customInstructions.description")} title={t("personalization.customInstructions.title")} variant="settings">
      <SettingsRow description={t("personalization.customInstructions.enabledDesc")} title={t("personalization.customInstructions.enabled")}>
        <button
          aria-checked={enabled}
          aria-label={t("personalization.customInstructions.enabled")}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${enabled ? "bg-primary" : "bg-muted-foreground/40"}`}
          disabled={busy}
          onClick={toggleEnabled}
          role="switch"
          type="button"
        >
          <span className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-background shadow-sm transition-transform ${enabled ? "translate-x-5" : "translate-x-0"}`} />
        </button>
      </SettingsRow>
      <div className="divide-y divide-border/70 border-t border-border/70">
        <CustomInstructionsField
          busy={busy || !enabled}
          label={t("personalization.customInstructions.styleRules")}
          onCommit={(value) => commit("customInstructionsStyleRules", value)}
          placeholder={t("personalization.customInstructions.styleRulesPlaceholder")}
          value={settings.customInstructionsStyleRules}
        />
        <CustomInstructionsField
          busy={busy || !enabled}
          label={t("personalization.customInstructions.aboutUser")}
          onCommit={(value) => commit("customInstructionsAboutUser", value)}
          placeholder={t("personalization.customInstructions.aboutUserPlaceholder")}
          value={settings.customInstructionsAboutUser}
        />
      </div>
    </SectionPanel>
  );
}
