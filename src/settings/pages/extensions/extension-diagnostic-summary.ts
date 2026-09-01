import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import type { ExtensionFrameworkDefinition, ExtensionFrameworkStatus } from "../../../types/extension";

/**
 * spec.md "Copyable safe settings diagnostics" for one extension framework card.
 *
 * Audited before writing this (task 12.19): a full read of `ExtensionFrameworkDefinition`,
 * `ExtensionRequirement`, `ExtensionFrameworkStatus`, `ExtensionEnvironment`, and
 * `ExtensionInstallPreview` (`src/types/extension.ts`) found no credential-shaped field anywhere
 * -- this page installs and runs local runtime frameworks (PaddleOCR, faster-whisper, sherpa-onnx)
 * the same way Local Media configures local OCR/STT/TTS engines, and every value below is already
 * a stable id, a backend-pinned enum/reason-code union, a plain boolean/number, a local filesystem
 * path, a version string, or a raw timestamp -- the same "everything is safe" shape CLI Management
 * and Local Media both have, unlike IM/SSH/Observability/OnePiece. There is nothing to redact on
 * this page. `status.lastError` looks free-text but isn't: `web-extension-client.ts` populates it
 * with a fixed i18n key (e.g. `"extensions.environment.desktopOnly"`), the same "backend-pinned
 * reason code, kept raw rather than translated for pasting" category CLI's own conflict/action
 * reason codes already use, not the raw free-text CLI's own doc comment says to exclude.
 *
 * Wired per-card (`extension-framework-card.tsx`), not page-level: unlike Local Media's one
 * combined profile spanning all three engines with a shared draft/dirty state, each framework here
 * already has its own independent install/start/stop/self-test/enable/uninstall lifecycle and its
 * own per-card `ActionMenu` (task 12.18) -- the same self-contained-per-entity shape IM's
 * per-connector rows have, not Local Media's single-page shape. `ExtensionEnvironment` (native
 * availability, Python discovery) is deliberately left out of this per-card builder: it is
 * identical across every card (a page-level fact, already surfaced once via the page's own
 * desktop-only banner in `extensions-page.tsx`), so repeating it on all three cards would not
 * uniquely diagnose any one framework, and would require threading a new page-level prop into this
 * card only for this feature. Also left out, for the same "not already in this card's own data"
 * reason: `ExtensionInstallPreview` (a transient, on-demand dialog fetched only after clicking
 * Requirements/Install, never data this card already holds) and the framework's own
 * `nameKey`/`descriptionKey` (translated, page-navigation text, not diagnostic facts --
 * `frameworkId` already identifies which framework this is, the same way CLI's `agentId` stands in
 * for a name rather than a translated display string).
 */
export function buildExtensionDiagnosticFields(
  definition: ExtensionFrameworkDefinition,
  status: ExtensionFrameworkStatus,
  t: TFunction,
): DiagnosticField[] {
  const packages = definition.requirement.packages;

  return [
    { label: t("extensions.diagnostics.field.frameworkId"), value: status.frameworkId },
    { label: t("extensions.diagnostics.field.capabilityId"), value: status.capabilityId },
    { label: t("extensions.diagnostics.field.installedVersion"), value: status.installedVersion },
    { label: t("extensions.diagnostics.field.status"), value: status.status },
    { label: t("extensions.diagnostics.field.installed"), value: String(status.installed) },
    { label: t("extensions.diagnostics.field.enabled"), value: String(status.enabled) },
    { label: t("extensions.diagnostics.field.running"), value: String(status.running) },
    { label: t("extensions.runtime"), value: definition.requirement.runtime },
    { label: t("extensions.port"), value: String(status.port) },
    { label: t("extensions.disk"), value: String(definition.requirement.estimatedDiskMb) },
    { label: t("extensions.diagnostics.field.packages"), value: packages.length > 0 ? packages.join(", ") : null },
    { label: t("extensions.diagnostics.field.installPath"), value: status.installPath },
    { label: t("extensions.diagnostics.field.lastHealthCheck"), value: status.lastHealthCheck },
    { label: t("extensions.diagnostics.field.lastOperationId"), value: status.lastOperationId },
    { label: t("extensions.diagnostics.field.lastError"), value: status.lastError },
  ];
}
