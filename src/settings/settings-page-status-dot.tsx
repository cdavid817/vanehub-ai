import { useTranslation } from "react-i18next";
import type { SettingsPageStatus, SettingsPageStatusKind } from "./settings-page-types";

/** Same `--color-*` tokens `StatusBadge` (`ui/status/StatusBadge.tsx`) renders as full pills --
 *  applied directly as Tailwind utilities here because a nav row has no room for that component's
 *  border, padding, and mandatory visible label; a plain `bg-*` avoids the unlayered `.ucd-status-*`
 *  classes stepping on it (those set their own `background`, meant for a soft-tinted pill, not a
 *  small solid dot). */
const DOT_CLASS: Record<SettingsPageStatusKind, string> = {
  error: "bg-danger",
  "dependency-unavailable": "bg-blocked",
  unsaved: "bg-attention",
  "restart-required": "bg-warning",
  "update-available": "bg-information",
};

/**
 * One bounded semantic dot for a settings nav entry (task 12.16, spec.md "Show page status").
 * The caller already resolved which single status to show (`pickPageStatus`) -- this only
 * renders it: a decorative dot plus a screen-reader-only accessible description, since the
 * indicator itself carries no visible text budget in a nav row.
 */
export function SettingsPageStatusDot({ status }: { status: SettingsPageStatus }) {
  const { t } = useTranslation();
  return (
    <span className="inline-flex shrink-0 items-center">
      <span aria-hidden="true" className={`h-2 w-2 rounded-full ${DOT_CLASS[status.kind]}`} />
      <span className="sr-only">{t(status.labelKey, status.labelParams)}</span>
    </span>
  );
}
