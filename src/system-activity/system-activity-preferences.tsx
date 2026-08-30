import { useTranslation } from "react-i18next";
import type { ReactNode } from "react";
import type {
  ActivityDigestCadence,
  ActivitySeverity,
  SystemActivityPreferences,
} from "../services/system-activity-service";

interface SystemActivityPreferencesPanelProps {
  preferences: SystemActivityPreferences;
  onSave: (next: SystemActivityPreferences) => void;
}

const severities: readonly ActivitySeverity[] = ["info", "warning", "error", "critical"];

export function SystemActivityPreferencesPanel({
  preferences,
  onSave,
}: SystemActivityPreferencesPanelProps) {
  const { t } = useTranslation();
  const updateNumber = (
    field: "readRetentionDays" | "detailRetentionDays" | "exportItemLimit" | "exportSizeLimitBytes",
    value: string,
    minimum: number,
    maximum: number,
  ) => {
    const parsed = Number.parseInt(value, 10);
    if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) return;
    onSave({ ...preferences, [field]: parsed });
  };

  return (
    <div className="space-y-2 text-xs" data-testid="system-activity-preferences">
      <label className="flex items-center gap-2">
        <input
          checked={preferences.visible}
          onChange={(event) => onSave({ ...preferences, visible: event.target.checked })}
          type="checkbox"
        />
        {t("systemActivity.view.preferenceVisible")}
      </label>
      <PreferenceSelect
        label={t("systemActivity.view.preferenceTimelineSeverity")}
        onChange={(value) => onSave({ ...preferences, minimumTimelineSeverity: value as ActivitySeverity })}
        value={preferences.minimumTimelineSeverity}
      >
        {severities.map((severity) => (
          <option key={severity} value={severity}>{t(`systemActivity.severity.${severity}.title`)}</option>
        ))}
      </PreferenceSelect>
      <PreferenceSelect
        label={t("systemActivity.view.preferenceNotificationThreshold")}
        onChange={(value) => onSave({ ...preferences, notificationThreshold: value as ActivitySeverity })}
        value={preferences.notificationThreshold}
      >
        {severities.map((severity) => (
          <option key={severity} value={severity}>{t(`systemActivity.severity.${severity}.title`)}</option>
        ))}
      </PreferenceSelect>
      <PreferenceSelect
        label={t("systemActivity.view.preferenceDigest")}
        onChange={(value) => onSave({ ...preferences, digestCadence: value as ActivityDigestCadence })}
        value={preferences.digestCadence}
      >
        <option value="off">{t("systemActivity.view.digestOff")}</option>
        <option value="hourly">{t("systemActivity.view.digestHourly")}</option>
        <option value="daily">{t("systemActivity.view.digestDaily")}</option>
      </PreferenceSelect>
      <PreferenceNumber
        key={`read-${preferences.revision}`}
        defaultValue={preferences.readRetentionDays}
        label={t("systemActivity.view.preferenceReadRetention")}
        max={365}
        min={30}
        onCommit={(value) => updateNumber("readRetentionDays", value, 30, 365)}
      />
      <PreferenceNumber
        key={`detail-${preferences.revision}`}
        defaultValue={preferences.detailRetentionDays}
        label={t("systemActivity.view.preferenceRetention")}
        max={365}
        min={30}
        onCommit={(value) => updateNumber("detailRetentionDays", value, 30, 365)}
      />
      <PreferenceNumber
        key={`items-${preferences.revision}`}
        defaultValue={preferences.exportItemLimit}
        label={t("systemActivity.view.preferenceExportItems")}
        max={10_000}
        min={1}
        onCommit={(value) => updateNumber("exportItemLimit", value, 1, 10_000)}
      />
      <PreferenceNumber
        key={`bytes-${preferences.revision}`}
        defaultValue={preferences.exportSizeLimitBytes}
        label={t("systemActivity.view.preferenceExportSize")}
        max={100 * 1024 * 1024}
        min={1}
        onCommit={(value) => updateNumber("exportSizeLimitBytes", value, 1, 100 * 1024 * 1024)}
      />
      <p className="text-[11px] leading-relaxed text-muted-foreground">
        {t("systemActivity.view.preferenceRetentionDisclosure")}
      </p>
    </div>
  );
}

function PreferenceSelect(props: {
  children: ReactNode;
  label: string;
  onChange: (value: string) => void;
  value: string;
}) {
  return (
    <label className="flex items-center justify-between gap-2">
      <span>{props.label}</span>
      <select
        aria-label={props.label}
        className="max-w-28 rounded-md border border-border bg-background px-1 py-0.5"
        onChange={(event) => props.onChange(event.target.value)}
        value={props.value}
      >
        {props.children}
      </select>
    </label>
  );
}

function PreferenceNumber(props: {
  defaultValue: number;
  label: string;
  max: number;
  min: number;
  onCommit: (value: string) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-2">
      <span>{props.label}</span>
      <input
        aria-label={props.label}
        className="w-24 rounded-md border border-border bg-background px-1 py-0.5 text-right tabular-nums"
        defaultValue={props.defaultValue}
        max={props.max}
        min={props.min}
        onBlur={(event) => props.onCommit(event.currentTarget.value)}
        type="number"
      />
    </label>
  );
}
