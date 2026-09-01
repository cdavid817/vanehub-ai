import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationState,
  PluginIntegrationTestResult,
} from "../../../types/plugin-integration";

/**
 * spec.md "Copyable safe settings diagnostics" for one plugin integration card. A full read of
 * `PluginIntegrationDefinition`/`PluginIntegrationState`/`PluginIntegrationTestResult` and the Rust
 * side that produces them (`plugin_integrations::domain::lifecycle::evaluate_readiness`) found no
 * credential-shaped field anywhere: `message`/`statusReasonKey` are always one of a small fixed set
 * of i18n reason keys (`PluginIntegrationStatus::reason_key()`), never the raw `gh auth status`
 * stdout/stderr the native tool adapter actually captures -- that raw process output is classified
 * into a status enum and discarded on the Rust side, never serialized to the frontend at all. Every
 * field below is therefore unconditionally safe, the same "nothing to redact" shape CLI Management's
 * own `CliEnvironmentSnapshot` diagnostics has.
 *
 * Deliberately excludes `definition.docsUrl` and `.setupSteps`: static catalog constants identical
 * for every user in every environment, so unlike CLI Management's `executablePath` (which varies by
 * installation and is already shown on screen) they carry no information about this user's own
 * state. Excludes translated `nameKey`/`descriptionKey` display text as a *value* for the same
 * reason every other page's builder keeps values untranslated -- `definition.id` already covers the
 * "stable id" category. Excludes `PluginIntegrationEnvironment` (`nativeChecksAvailable`/`runtime`/
 * `reasonKey`): it belongs to the whole page, not to any one integration's own card, and this page's
 * own banner already surfaces it separately -- matching IM's own precedent of omitting a category
 * that does not cleanly belong to a per-item builder rather than reaching outside the item for it.
 * No "safe path" category either: unlike CLI Management or Local Media, nothing about a plugin
 * integration is a filesystem path the frontend ever sees (the Rust side keeps its `gh` executable
 * name and arguments internal, never serialized to the DTO).
 *
 * `statusReason` mirrors `PluginIntegrationCard`'s own `messageKey` precedence (a freshly-completed
 * test's own message wins over the last-known static reason) so the copied text matches what the
 * card currently shows on screen, not a second, silently different source of truth.
 */
export function buildPluginIntegrationDiagnosticFields(
  definition: PluginIntegrationDefinition,
  state: PluginIntegrationState,
  lastResult: PluginIntegrationTestResult | undefined,
  t: TFunction,
): DiagnosticField[] {
  const statusReason = lastResult?.integrationId === definition.id ? lastResult.message : state.statusReasonKey;

  return [
    { label: t("plugins.diagnostics.field.id"), value: definition.id },
    { label: t("plugins.diagnostics.field.provider"), value: definition.provider },
    { label: t("plugins.diagnostics.field.version"), value: definition.version },
    { label: t("plugins.diagnostics.field.status"), value: state.status },
    { label: t("plugins.diagnostics.field.configured"), value: String(state.configured) },
    { label: t("plugins.diagnostics.field.canTest"), value: String(state.canTest) },
    { label: t("plugins.diagnostics.field.lastCheckedAt"), value: state.lastCheckedAt },
    { label: t("plugins.diagnostics.field.statusReason"), value: statusReason },
  ];
}
