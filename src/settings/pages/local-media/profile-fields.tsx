import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "../../../components/ui/button";
import { localMediaService } from "../../../services/runtime-local-media-client";

const CONTROL_CLASS =
  "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring";
const INVALID_CLASS = "border-danger";

/**
 * One labelled row with an optional field error.
 *
 * The error is rendered from a locale key the native validator supplied, not from a message the
 * page composed: the same rule may be enforced natively long after the page stops looking, and two
 * independently worded copies of "this path is not usable" would drift.
 */
function FieldShell({
  children,
  htmlFor,
  hintKey,
  issueKey,
  label,
}: {
  children: ReactNode;
  htmlFor: string;
  hintKey?: string;
  issueKey?: string;
  label: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-1.5">
      <label className="text-xs font-medium leading-5 text-foreground" htmlFor={htmlFor}>
        {label}
      </label>
      {children}
      {hintKey ? <p className="text-xs leading-5 text-muted-foreground">{t(hintKey)}</p> : null}
      {issueKey ? (
        <p className="text-xs leading-5 text-danger" id={`${htmlFor}-error`} role="alert">
          {t(issueKey)}
        </p>
      ) : null}
    </div>
  );
}

export function TextField({
  hintKey,
  id,
  issueKey,
  label,
  onChange,
  value,
}: {
  hintKey?: string;
  id: string;
  issueKey?: string;
  label: string;
  onChange: (value: string) => void;
  value: string;
}) {
  return (
    <FieldShell hintKey={hintKey} htmlFor={id} issueKey={issueKey} label={label}>
      <input
        aria-describedby={issueKey ? `${id}-error` : undefined}
        aria-invalid={issueKey ? true : undefined}
        className={`${CONTROL_CLASS} ${issueKey ? INVALID_CLASS : ""}`}
        id={id}
        onChange={(event) => onChange(event.currentTarget.value)}
        type="text"
        value={value}
      />
    </FieldShell>
  );
}

/**
 * A path input paired with the native picker.
 *
 * The input stays editable. A picker alone would be unusable for a headless model directory the
 * user reached over a mount, and typing is also how a profile gets fixed after moving machines.
 */
export function PathField({
  disabled,
  hintKey,
  id,
  issueKey,
  kind,
  label,
  onChange,
  optional = false,
  value,
}: {
  disabled?: boolean;
  hintKey?: string;
  id: string;
  issueKey?: string;
  kind: "file" | "directory";
  label: string;
  onChange: (value: string) => void;
  optional?: boolean;
  value: string;
}) {
  const { t } = useTranslation();
  return (
    <FieldShell hintKey={hintKey} htmlFor={id} issueKey={issueKey} label={label}>
      <div className="flex items-center gap-2">
        <input
          aria-describedby={issueKey ? `${id}-error` : undefined}
          aria-invalid={issueKey ? true : undefined}
          className={`${CONTROL_CLASS} ${issueKey ? INVALID_CLASS : ""}`}
          disabled={disabled}
          id={id}
          onChange={(event) => onChange(event.currentTarget.value)}
          spellCheck={false}
          type="text"
          value={value}
        />
        <Button
          className="shrink-0"
          disabled={disabled}
          onClick={() => {
            void localMediaService.selectProfilePath({ kind }).then((picked) => {
              if (picked) onChange(picked);
            });
          }}
          size="sm"
          type="button"
          variant="outline"
        >
          {t("localMedia.settings.browse")}
        </Button>
        {optional && value.length > 0 ? (
          <Button
            className="shrink-0"
            disabled={disabled}
            onClick={() => onChange("")}
            size="sm"
            type="button"
            variant="ghost"
          >
            {t("localMedia.settings.clear")}
          </Button>
        ) : null}
      </div>
    </FieldShell>
  );
}

export function NumberField({
  hintKey,
  id,
  issueKey,
  label,
  max,
  min,
  onChange,
  step,
  value,
}: {
  hintKey?: string;
  id: string;
  issueKey?: string;
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  step?: number;
  value: number;
}) {
  return (
    <FieldShell hintKey={hintKey} htmlFor={id} issueKey={issueKey} label={label}>
      <input
        aria-describedby={issueKey ? `${id}-error` : undefined}
        aria-invalid={issueKey ? true : undefined}
        className={`${CONTROL_CLASS} ${issueKey ? INVALID_CLASS : ""}`}
        id={id}
        max={max}
        min={min}
        onChange={(event) => {
          const parsed = Number(event.currentTarget.value);
          // An empty or half-typed value parses to NaN; keeping the previous number is better than
          // writing NaN into the profile and failing validation on a field the user is mid-edit on.
          if (Number.isFinite(parsed)) onChange(parsed);
        }}
        step={step ?? 1}
        type="number"
        value={value}
      />
    </FieldShell>
  );
}

export function SelectField<Value extends string>({
  hintKey,
  id,
  issueKey,
  label,
  onChange,
  options,
  value,
}: {
  hintKey?: string;
  id: string;
  issueKey?: string;
  label: string;
  onChange: (value: Value) => void;
  options: ReadonlyArray<{ value: Value; labelKey: string }>;
  value: Value;
}) {
  const { t } = useTranslation();
  return (
    <FieldShell hintKey={hintKey} htmlFor={id} issueKey={issueKey} label={label}>
      <select
        aria-describedby={issueKey ? `${id}-error` : undefined}
        aria-invalid={issueKey ? true : undefined}
        className={`${CONTROL_CLASS} ${issueKey ? INVALID_CLASS : ""}`}
        id={id}
        onChange={(event) => onChange(event.currentTarget.value as Value)}
        value={value}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {t(option.labelKey)}
          </option>
        ))}
      </select>
    </FieldShell>
  );
}

/** Devices come from the host, so their labels are data and are never translated. */
export function DeviceField({
  devices,
  emptyKey,
  id,
  label,
  onChange,
  value,
}: {
  devices: ReadonlyArray<{ deviceId: string; label: string; isDefault: boolean }>;
  emptyKey: string;
  id: string;
  label: string;
  onChange: (value: string | null) => void;
  value: string | null;
}) {
  const { t } = useTranslation();
  return (
    <FieldShell htmlFor={id} label={label}>
      <select
        className={CONTROL_CLASS}
        id={id}
        onChange={(event) => onChange(event.currentTarget.value === "" ? null : event.currentTarget.value)}
        value={value ?? ""}
      >
        <option value="">{t("localMedia.settings.systemDefault")}</option>
        {devices.map((device) => (
          <option key={device.deviceId} value={device.deviceId}>
            {device.label}
          </option>
        ))}
      </select>
      {devices.length === 0 ? (
        <p className="text-xs leading-5 text-muted-foreground">{t(emptyKey)}</p>
      ) : null}
    </FieldShell>
  );
}

export function ToggleField({
  checked,
  label,
  onChange,
}: {
  checked: boolean;
  label: string;
  onChange: (value: boolean) => void;
}) {
  const { t } = useTranslation();
  return (
    <Button
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      role="switch"
      size="sm"
      type="button"
      variant={checked ? "default" : "outline"}
    >
      {t(checked ? "localMedia.settings.enabled" : "localMedia.settings.disabled")}
    </Button>
  );
}
