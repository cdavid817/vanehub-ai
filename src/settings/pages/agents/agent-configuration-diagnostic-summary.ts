import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import {
  managedSettingsPath,
  payloadSupportsEndpointOverride,
  type CliConfigAgentId,
  type CliConfigPayload,
  type CliConfigProfile,
  type CliConfigStatus,
} from "../../../types/cli-agent-config";
import type { OnePieceProviderProfile, OnePieceProviderProfiles } from "../../../types/agent";

/**
 * spec.md "Copyable safe settings diagnostics" for the Agent Configurations page. One selector
 * (`AgentConfigurationSelector`) hides two genuinely different data shapes, audited separately
 * rather than assumed from either the CLI Management or IM precedent:
 *
 * **CLI-agent branch** (`AgentGlobalConfigPanel`, `CliConfigProfile`/`CliConfigStatus`): closer to
 * CLI Management's own shape than IM's, but not identical -- unlike `CliEnvironmentSnapshot`, this
 * page's own `SaveCliConfigProfileInput.credential` write-only field proves a credential concept
 * exists here too, and `CliConfigProfile.credentialConfigured: boolean` is the already-existing
 * flag-not-value signal for it (same pattern as SSH's `hasPassword`/IM's `hasCredentials`/
 * Observability's `otlpAuthConfigured`) -- reused as-is, nothing new invented. Three categories of
 * field were excluded after reading every payload-editing component in full
 * (`cli-config-payload-fields.tsx`, `cli-config-profile-list.tsx`), not assumed safe by shape alone:
 * (1) `name` (the profile's own free-text label) -- excluded on the exact ground
 * `ssh-connection-diagnostic-summary.ts` already established for the same shape of field ("the same
 * 'user-authored description' category `diagnostic-field.ts`'s own doc comment already excludes"),
 * with `profile.id` reused as this summary's own "which profile is this" correlating field in its
 * place, matching that same precedent; (2) `providerId` (codex-cli) and `providerName` (opencode) --
 * both look like stable catalog ids at a glance, but a read of `CodexFields`/`OpenCodeFields` shows
 * both are plain, live-editable `<input>` text fields a user can retype freely after a preset seeds
 * them, not a backend-pinned id, so they get the same treatment as `name`; (3) every payload kind's
 * own `advancedEnv`/`advancedToml`/`headers`/`advancedSettings` free-form record -- unbounded,
 * user-typed key/value content (spec.md's own "environment values" exclusion category by name, and
 * the same "a user can type arbitrary content into it" ground LSP's own `initializationOptions`
 * exclusion already used), so a custom header or env var a user pastes a credential into can never
 * leak through this builder. `startupSync.warnings` is excluded too, on `lastError`'s own precedent
 * from the SSH file: unbounded free text, and this page already has real bounded remediation-code
 * equivalents (`driftState`, `validationState`, `appliedState`, `startupSync.state`) to report
 * instead, so nothing is lost by leaving the prose out. `baseUrl` is kept, unlike SSH's `host`: SSH
 * excluded `host` as a private external identifier of one user's own machine, but a CLI agent's
 * `baseUrl` is a provider API endpoint -- the same publicly-shown-on-screen, non-personal shape
 * OnePiece's own `baseUrl` already is (rendered unmasked next to the profile name in
 * `OnePieceConfigurationPanel`, never behind a reveal toggle).
 *
 * **OnePiece branch** (`OnePieceConfigurationPanel`): this is the hard case task 12.19's own audit
 * named by name -- `apiKey` is confirmed write-only by reading every type that carries it
 * (`SaveOnePieceProviderConfigInput`, `SaveOnePieceProviderProfileInput`,
 * `SaveCustomOnePieceProviderProfileInput`, `ValidateOnePieceProviderCredentialInput`,
 * `DiscoverOnePieceProviderModelsInput`): every one is a mutation *input* type, and `apiKey` never
 * appears on any query *result* type this page reads (`OnePieceProviderProfile`,
 * `OnePieceProviderProfiles`). A real, already-server-confirmed boolean already exists in its
 * place: `OnePieceProviderProfile.credentialPresent`, the exact field
 * `OnePieceConfigurationPanel`'s own status banner and `KeyRound` row already branch on today --
 * reused as this summary's `credentialConfigured` value, not a client-side approximation invented
 * for this task (a draft's local, momentarily-non-empty `apiKey` input state would have been exactly
 * the kind of guess this task's own rule forbids; it was never needed here). `name` is excluded on
 * the same free-text ground as the CLI branch (confirmed via `OnePieceProviderDialog`'s own plain
 * `<input>`); `profile.provider`, unlike `name`, is kept -- tracing `OnePieceProviderDialog`'s save
 * call shows it sends `providerId` (the selected catalog preset's id), never a free-text "provider"
 * string, so the resulting `profile.provider` the backend returns is catalog-derived, not
 * user-typed.
 *
 * **Deliberately out of scope for this pass, not silently dropped**: `OnePieceParametersPanel`'s own
 * three sections (retrieval configuration, automatic context compaction, context quality
 * health/history) and `OnePieceConfigurationPanel`'s own "runtime" (`HybridLocalRuntimeSection`) and
 * "tools" (`OnePieceToolReadiness`) tabs were all read before writing this file. None holds a
 * credential-shaped field the provider-profile case above doesn't already cover -- except hybrid
 * runtime's own custom local/private endpoints, which have their *own* write-only `apiKey`
 * (`hybrid-local-runtime-section.tsx`) but, unlike a provider profile, no equivalent confirmed
 * boolean anywhere on `EndpointProfileMetadata` standing in for it. That is a genuinely separate
 * redaction judgment call this task's own scope centers on the write-only provider-profile `apiKey`
 * case, not an exhaustive sweep of every OnePiece setting -- covering it here would be a rushed
 * guess, not the deliberate pass it deserves.
 */

function profileField(t: TFunction, profileId: string, field: string): string {
  return t("agentConfigurations.diagnostics.field.profilePrefixed", { profile: profileId, field });
}

function cliProfilePayloadFields(t: TFunction, payload: CliConfigPayload): DiagnosticField[] {
  const kindFields: DiagnosticField[] =
    payload.kind === "claude-code"
      ? [{ label: t("agentConfigurations.diagnostics.field.authMode"), value: payload.authMode }]
      : payload.kind === "codex-cli"
        ? [
            { label: t("agentConfigurations.diagnostics.field.wireApi"), value: payload.wireApi },
            { label: t("agentConfigurations.diagnostics.field.reasoningEffort"), value: payload.reasoningEffort },
            { label: t("agentConfigurations.diagnostics.field.authStrategy"), value: payload.authStrategy },
          ]
        : payload.kind === "gemini-cli"
          ? [{ label: t("agentConfigurations.diagnostics.field.authStrategy"), value: payload.authStrategy }]
          : payload.kind === "antigravity"
            ? [
                { label: t("agentConfigurations.diagnostics.field.toolPermission"), value: payload.toolPermission },
                { label: t("agentConfigurations.diagnostics.field.terminalSandboxEnabled"), value: String(payload.enableTerminalSandbox) },
              ]
            // opencode has no bounded auth/permission enum of its own beyond endpoint + model below;
            // its own `providerId`/`providerName`/`headers` are excluded per this file's own doc comment.
            : [];

  const endpoint = payloadSupportsEndpointOverride(payload) ? (payload.baseUrl || null) : managedSettingsPath(payload);
  const model = payload.kind === "opencode" ? payload.defaultModel : payload.model;

  return [
    { label: t("agentConfigurations.diagnostics.field.payloadKind"), value: payload.kind },
    { label: t("agentConfigurations.diagnostics.field.endpoint"), value: endpoint || null },
    { label: t("agentConfigurations.diagnostics.field.model"), value: model || null },
    ...kindFields,
  ];
}

function cliProfileDiagnosticFields(t: TFunction, profile: CliConfigProfile): DiagnosticField[] {
  const own: DiagnosticField[] = [
    { label: t("agentConfigurations.diagnostics.field.payloadVersion"), value: String(profile.payloadVersion) },
    { label: t("agentConfigurations.diagnostics.field.sourcePresetId"), value: profile.sourcePresetId },
    { label: t("agentConfigurations.diagnostics.field.sourcePresetVersion"), value: profile.sourcePresetVersion !== null ? String(profile.sourcePresetVersion) : null },
    { label: t("agentConfigurations.diagnostics.field.credentialConfigured"), value: String(profile.credentialConfigured) },
    { label: t("agentConfigurations.diagnostics.field.validationState"), value: profile.validationState },
    { label: t("agentConfigurations.diagnostics.field.appliedState"), value: profile.appliedState },
    { label: t("agentConfigurations.diagnostics.field.createdAt"), value: profile.createdAt },
    { label: t("agentConfigurations.diagnostics.field.updatedAt"), value: profile.updatedAt },
    ...cliProfilePayloadFields(t, profile.payload),
  ];
  return own.map((field) => ({ label: profileField(t, profile.id, field.label), value: field.value }));
}

/** For the CLI-agent branch: `AgentGlobalConfigPanel`'s own status card plus every saved profile. */
export function buildCliAgentConfigDiagnosticFields(
  agentId: CliConfigAgentId,
  status: CliConfigStatus | undefined,
  profiles: readonly CliConfigProfile[],
  t: TFunction,
): DiagnosticField[] {
  const resolvedPaths = status?.resolvedPaths ?? [];

  return [
    { label: t("agentConfigurations.diagnostics.field.agentId"), value: agentId },
    { label: t("agentConfigurations.diagnostics.field.driftState"), value: status?.driftState ?? null },
    { label: t("agentConfigurations.diagnostics.field.appliedProfileId"), value: status?.appliedProfileId ?? null },
    { label: t("agentConfigurations.diagnostics.field.simulated"), value: status ? String(status.simulated) : null },
    { label: t("agentConfigurations.status.paths"), value: resolvedPaths.length ? resolvedPaths.join(", ") : null },
    { label: t("agentConfigurations.status.lastApplied"), value: status?.lastAppliedAt ?? null },
    { label: t("agentConfigurations.diagnostics.field.startupSyncState"), value: status?.startupSync.state ?? null },
    { label: t("agentConfigurations.diagnostics.field.startupSyncImported"), value: status ? String(status.startupSync.imported) : null },
    { label: t("agentConfigurations.diagnostics.field.startupSyncUpdated"), value: status ? String(status.startupSync.updated) : null },
    { label: t("agentConfigurations.diagnostics.field.startupSyncSkipped"), value: status ? String(status.startupSync.skipped) : null },
    { label: t("agentConfigurations.diagnostics.field.startupSyncedAt"), value: status?.startupSync.synchronizedAt ?? null },
    { label: t("agentConfigurations.diagnostics.field.startupSyncSimulated"), value: status ? String(status.startupSync.simulated) : null },
    ...profiles.flatMap((profile) => cliProfileDiagnosticFields(t, profile)),
  ];
}

function onePieceProfileDiagnosticFields(t: TFunction, profile: OnePieceProviderProfile): DiagnosticField[] {
  const own: DiagnosticField[] = [
    { label: t("agentConfigurations.diagnostics.field.provider"), value: profile.provider },
    { label: t("agentConfigurations.diagnostics.field.sourceProviderId"), value: profile.sourceProviderId },
    { label: t("agentConfigurations.diagnostics.field.sourceEndpointType"), value: profile.sourceEndpointType },
    { label: t("agentConfigurations.diagnostics.field.sourcePresetVersion"), value: profile.sourcePresetVersion !== null ? String(profile.sourcePresetVersion) : null },
    { label: t("agentConfigurations.diagnostics.field.interfaceFormat"), value: profile.interfaceFormat },
    { label: t("agentConfigurations.diagnostics.field.endpoint"), value: profile.baseUrl || null },
    { label: t("agentConfigurations.diagnostics.field.model"), value: profile.modelId || null },
    { label: t("agentConfigurations.diagnostics.field.active"), value: String(profile.active) },
    // The write-only apiKey boundary: `credentialPresent` is the real, already-server-confirmed
    // flag this page's own status banner branches on -- never the apiKey value itself, which this
    // function never even receives (`OnePieceProviderProfile` has no such field to begin with).
    { label: t("agentConfigurations.diagnostics.field.credentialConfigured"), value: String(profile.credentialPresent) },
  ];
  return own.map((field) => ({ label: profileField(t, profile.id, field.label), value: field.value }));
}

/** For the OnePiece branch: `OnePieceConfigurationPanel`'s own provider-profiles overview. */
export function buildOnePieceConfigDiagnosticFields(overview: OnePieceProviderProfiles | undefined, t: TFunction): DiagnosticField[] {
  const profiles = overview?.profiles ?? [];

  return [
    { label: t("agentConfigurations.diagnostics.field.agentId"), value: "onepiece" },
    { label: t("agentConfigurations.diagnostics.field.activeProfileId"), value: overview?.activeProfileId ?? null },
    ...profiles.flatMap((profile) => onePieceProfileDiagnosticFields(t, profile)),
  ];
}
