import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import type {
  LspConfiguration,
  LspLanguageConfiguration,
  LspLanguageDescriptor,
  LspServerDiscovery,
  LspServerStatus,
  LspWorkspaceTrust,
} from "../../../types/lsp";

/**
 * spec.md "Copyable safe settings diagnostics" for the Code Intelligence / LSP page.
 *
 * Audited before writing this (task 12.19's own deferred note flagged this page as having "the
 * strongest existing remediation-code precedent in the app via its own `LspSafeReasonCode`"): a
 * full read of `types/lsp.ts` confirms there is no credential-shaped field anywhere in this page's
 * data model either -- unlike IM/SSH, nothing here is excluded for being a *secret*. What is
 * excluded is excluded for being unbounded free text or JSON (`initializationOptions`,
 * `startupArguments`, the registry's own `defaultStartupArguments`, and discovery's resolved
 * `arguments` -- none of these are rendered as a single bounded value anywhere, and a user can type
 * arbitrary content into the first two) or, matching Local Media's own precedent, for not being
 * rendered anywhere on this page at all (`LspWorkspaceTrust.revision`; a revoked or never-trusted
 * workspace's own record, which drops out of `LspWorkspaceTrustPanel`'s own list entirely once
 * revoked; the per-language `installReasons` / install-busy transient state `LspConfigurationSection`
 * owns locally; and each server-test card's own transient `mutation.data` -- all in-progress UI
 * feedback, not a fact about the environment, the same category Local Media's own `issues` map was
 * excluded under).
 *
 * Every `LspSafeReasonCode` here is kept as its raw, untranslated wire value (never passed through
 * `t(\`lspSettings.reason.${code}\`)`), matching `cli-diagnostic-summary.ts`'s own established
 * precedent for this exact kind of union: a pasted diagnostic needs the stable, grep-able code, not
 * a locale-dependent paraphrase of it.
 *
 * Unlike CLI Management/Local Media, no single component here already holds every field this
 * builder needs as a prop -- the page's own 4 sections (`LspConfigurationSection`,
 * `LspWorkspaceTrustPanel`, `LspServerTestPanel`, `LspRuntimeStatusPanel`) each run their own
 * `useQuery`. `CodeIntelligencePage` calls this with its own read of the same 4 exported query keys
 * -- the same cross-component key-sharing this page's own sections already rely on
 * (`LspServerTestPanel` already reuses `lspConfigurationQueryKey` rather than fetching its own copy)
 * -- rather than a new architecture.
 *
 * Two distinct prefix wrappers, not one: a configured *language* (`descriptors`, bounded, static per
 * session) and a running *instance* (`statuses`, dynamic, keyed by language+server+project root, so
 * two simultaneous instances of the same language against different trusted workspaces need
 * distinguishing) are different grouping axes, unlike Local Media's single fixed OCR/STT/TTS engine
 * axis -- collapsing both into one generically-named wrapper key would be less legible to a
 * translator than naming each for what it actually groups.
 */
function languageDisplayName(t: TFunction, language: string): string {
  // Falls back to the raw id, matching every LSP component's own established fallback (a language
  // added to the backend renders here rather than going blank in a stale locale).
  return t(`lspSettings.language.${language}`, { defaultValue: language });
}

function languageField(t: TFunction, languageName: string, field: string): string {
  return t("lspSettings.diagnostics.field.languagePrefixed", { language: languageName, field });
}

function instanceField(t: TFunction, instance: string, field: string): string {
  return t("lspSettings.diagnostics.field.instancePrefixed", { instance, field });
}

function languageDiagnosticFields(
  t: TFunction,
  descriptor: LspLanguageDescriptor,
  languageConfig: LspLanguageConfiguration | undefined,
  discovery: LspServerDiscovery | undefined,
): DiagnosticField[] {
  const languageName = languageDisplayName(t, descriptor.language);
  // Matches `LspLanguageConfigurationCard`'s own conditional label choice exactly, not a fresh
  // redaction judgment -- an install-directory language's override is a directory, not a file.
  const overrideLabel = descriptor.overrideTarget === "install_directory"
    ? t("lspSettings.discovery.installDirectory")
    : t("lspSettings.discovery.override");

  return [
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.enabled")), value: languageConfig ? String(languageConfig.enabled) : null },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.supportedOnHost")), value: String(descriptor.supportedOnHost) },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.overrideTarget")), value: descriptor.overrideTarget },
    { label: languageField(t, languageName, overrideLabel), value: languageConfig?.executableOverride || null },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.discoveryAvailability")), value: discovery?.availability ?? null },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.discoveryExecutablePath")), value: discovery?.executablePath ?? null },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.discoveryReasonCode")), value: discovery?.reasonCode ?? null },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.installed")), value: String(descriptor.installed) },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.distributionVerified")), value: descriptor.distribution ? String(descriptor.distribution.verified) : null },
    { label: languageField(t, languageName, t("lspSettings.diagnostics.field.prerequisite")), value: descriptor.prerequisite },
  ];
}

function instanceDiagnosticFields(t: TFunction, status: LspServerStatus): DiagnosticField[] {
  const languageName = languageDisplayName(t, status.language);
  const instance = `${languageName} · ${status.server} · ${status.relativeProjectRoot}`;
  const capabilities = status.negotiatedCapabilities;

  return [
    { label: instanceField(t, instance, t("lspSettings.diagnostics.field.state")), value: status.state },
    { label: instanceField(t, instance, t("lspSettings.runtime.restartCount")), value: String(status.restartCount) },
    { label: instanceField(t, instance, t("lspSettings.runtime.lastResponse")), value: status.lastResponseAt },
    { label: instanceField(t, instance, t("lspSettings.runtime.diagnostics")), value: String(status.diagnosticCount) },
    { label: instanceField(t, instance, t("lspSettings.diagnostics.field.statusReasonCode")), value: status.reasonCode },
    { label: instanceField(t, instance, t("lspSettings.capability.positionEncoding")), value: capabilities?.positionEncoding ?? null },
    { label: instanceField(t, instance, t("lspSettings.capability.documentSync")), value: capabilities?.documentSync ?? null },
  ];
}

export function buildLspDiagnosticFields(
  configuration: LspConfiguration | undefined,
  discoveries: readonly LspServerDiscovery[],
  trustRecords: readonly LspWorkspaceTrust[],
  statuses: readonly LspServerStatus[],
  t: TFunction,
): DiagnosticField[] {
  // Only trusted roots: a revoked one drops out of `LspWorkspaceTrustPanel`'s own list entirely, so
  // it has nothing rendered anywhere on this page to reuse and no business appearing here either.
  const trustedRoots = trustRecords.filter((record) => record.trusted).map((record) => record.canonicalRoot);
  const descriptors = configuration?.descriptors ?? [];

  return [
    { label: t("lspSettings.configuration.master"), value: configuration ? String(configuration.enabled) : null },
    { label: t("lspSettings.diagnostics.field.trustedWorkspaceRoots"), value: trustedRoots.length ? trustedRoots.join(", ") : null },
    ...descriptors.flatMap((descriptor) => languageDiagnosticFields(
      t,
      descriptor,
      configuration?.languages.find((entry) => entry.language === descriptor.language),
      discoveries.find((entry) => entry.language === descriptor.language),
    )),
    ...statuses.flatMap((status) => instanceDiagnosticFields(t, status)),
  ];
}
