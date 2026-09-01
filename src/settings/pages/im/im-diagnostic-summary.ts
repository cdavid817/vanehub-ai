import type { TFunction } from "i18next";
import type { ImConnectorView } from "../../../contracts/im";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import { credentialFields } from "./im-form";

/**
 * spec.md "Copyable safe settings diagnostics" for one IM connector. Reuses the same
 * `credentialFields[kind].secret` manifest `credentialDraftAfterSave` (`im-form.ts`) already
 * trusts to tell a safe config value from a credential -- not a redaction judgment invented fresh
 * here. A `publicConfig` value that isn't a string/number/boolean (never expected per the
 * manifest, but not itself impossible) is treated as unavailable rather than stringified, since
 * this function has no way to know an unexpected shape doesn't itself carry something sensitive.
 *
 * No "version" or "safe path" category: connectors are not versioned software and have no path of
 * their own, so neither category applies to this page at all (spec.md: "supported by the page").
 */
export function buildImConnectorDiagnosticFields(view: ImConnectorView, t: TFunction): DiagnosticField[] {
  const kind = view.descriptor.kind;
  const safeFields = kind === "weixin" ? [] : credentialFields[kind].filter((field) => !field.secret);
  const safeConfigEntries: DiagnosticField[] = safeFields.map((field) => {
    const raw = view.config.publicConfig[field.key];
    const value = typeof raw === "string" || typeof raw === "number" || typeof raw === "boolean" ? String(raw) : null;
    return { label: t(`im.fields.${field.key}`), value };
  });

  return [
    { label: t("im.diagnostics.field.kind"), value: kind },
    { label: t("im.diagnostics.field.lifecycle"), value: view.health.lifecycle },
    { label: t("im.diagnostics.field.enabled"), value: String(view.config.enabled) },
    { label: t("im.diagnostics.field.hasCredentials"), value: String(view.hasCredentials) },
    ...safeConfigEntries,
    { label: t("im.diagnostics.field.updatedAt"), value: view.health.updatedAt },
    { label: t("im.diagnostics.field.safeErrorCode"), value: view.health.safeErrorCode ?? null },
  ];
}
