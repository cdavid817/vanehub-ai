import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../ui/diagnostics/diagnostic-field";
import type { ExecutionObservationCapability, ObservabilitySettings } from "../../types/execution-observability";

/**
 * spec.md "Copyable safe settings diagnostics" for the Execution Observability page. This page is
 * backed by two separate query results (`observability-settings-page.tsx`'s own `settingsQuery` and
 * `capabilitiesQuery`) rather than one combined object, so this builder takes both -- the same
 * multi-argument shape `local-media-diagnostic-summary.ts`'s `buildLocalMediaDiagnosticFields`
 * already established for a page backed by more than one query result.
 *
 * The one credential-shaped field in this page's data model is `ObservabilitySettings.otlpAuthToken`
 * -- task 12.19's own audit already named it explicitly ("must never appear in any page's summary")
 * before this file existed. It is write-only by contract (`execution-observability.ts`'s own doc
 * comment: "Native responses always omit this value"), and the page's own `supportedDraft()` already
 * nulls it out of the client-side draft immediately after every load and save. This builder goes
 * further and never reads that field at all, by construction -- not even to null-check it -- so a
 * caller that ever passed the raw (non-drafted) settings object still could not leak it here. In its
 * place, `otlpAuthConfigured` (a plain boolean `ObservabilitySettings` already exposes, and the same
 * field the page's own auth-field placeholder already branches on) is reported instead, the same
 * flag-not-value boundary SSH's `hasPassword` and IM's `hasCredentials` already established.
 *
 * `otlpProtocol` is excluded: `execution-observability.ts` pins it to a single literal
 * (`OtlpProtocol = "http_protobuf"`) today, and a full read of `observability-settings-page.tsx`'s
 * JSX confirms nothing on this page renders it anywhere -- matching SSH's own precedent for an
 * unrendered field ("including it would invent a new surface, not reuse one"), with no diagnostic
 * value today besides.
 *
 * `otlpEndpoint` is included as the plain string the page already shows in a plain-text (not
 * `type="password"`) input -- `validateObservabilitySettings` already rejects a URL with embedded
 * credentials (a username, password, or fragment), so nothing this page will accept here can itself
 * be a secret.
 *
 * From `ExecutionObservationCapability[]`: `relayAvailable` mirrors the page's own computed boolean
 * of the same name, which already drives the rendered `observability.mcp.available` /
 * `.unavailable` hint text -- deliberately unfiltered by transport, the same way the page's own
 * computation is. `relaySupportedAgentIds` / `opaqueAgentIds` mirror the per-agent grid rendered
 * immediately below that hint (each card already renders one `agentId` plus a relay/opaque badge
 * derived from `relaySupported`) -- filtered to `transport === "stdio"` only, the same filter the
 * grid itself applies, then joined the same way CLI's own `conflictCodes` / `actionCodes` join an
 * array of backend values into one copyable line. `transport`, `toolFidelity`, `mcpFidelity`, and
 * `detail` are left out: none of the four is rendered anywhere on this page (`detail` is a
 * backend-authored sentence, not a stable code -- the closest thing this page's data model has to
 * the unbounded free text `diagnostic-field.ts`'s own doc comment excludes), so including any would
 * invent a new surface rather than reuse one, the same "unrendered field stays out" rule this task
 * applies throughout.
 */
export function buildObservabilityDiagnosticFields(
  settings: ObservabilitySettings,
  capabilities: readonly ExecutionObservationCapability[],
  t: TFunction,
): DiagnosticField[] {
  const relayAvailable = capabilities.some((item) => item.relaySupported);
  const stdioCapabilities = capabilities.filter((item) => item.transport === "stdio");
  const relaySupportedAgentIds = stdioCapabilities.filter((item) => item.relaySupported).map((item) => item.agentId);
  const opaqueAgentIds = stdioCapabilities.filter((item) => !item.relaySupported).map((item) => item.agentId);

  return [
    { label: t("observability.local.enabled"), value: String(settings.localTimelineEnabled) },
    { label: t("observability.retention"), value: String(settings.retentionDays) },
    { label: t("observability.export.enabled"), value: String(settings.otlpEnabled) },
    { label: t("observability.export.endpoint"), value: settings.otlpEndpoint ?? null },
    { label: t("observability.export.sampling"), value: String(settings.samplingRatio) },
    { label: t("observability.diagnostics.field.otlpAuthConfigured"), value: String(settings.otlpAuthConfigured) },
    { label: t("observability.capture.policy"), value: settings.capturePolicy },
    { label: t("observability.mcp.relay"), value: String(settings.mcpRelayEnabled) },
    { label: t("observability.diagnostics.field.relayAvailable"), value: String(relayAvailable) },
    { label: t("observability.diagnostics.field.relaySupportedAgentIds"), value: relaySupportedAgentIds.length ? relaySupportedAgentIds.join(", ") : null },
    { label: t("observability.diagnostics.field.opaqueAgentIds"), value: opaqueAgentIds.length ? opaqueAgentIds.join(", ") : null },
  ];
}
