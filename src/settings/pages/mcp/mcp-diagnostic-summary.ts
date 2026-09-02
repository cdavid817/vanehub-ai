import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import type { McpServerConfig, McpServerStatus } from "../../../types/mcp";

/**
 * spec.md "Copyable safe settings diagnostics" for one MCP server card.
 *
 * `env`/`headers` are this page's own audit-named hard boundary (tasks.md 12.19: "must never
 * appear in any page's summary") and are never read here for their *values*, in full or in part.
 * `McpServerForm` (`mcp-server-form.tsx`) already treats both as credential-shaped on this exact
 * page -- an existing server's non-empty env/headers start masked behind an explicit Reveal button
 * ("Env/headers can carry real credentials... avoid displaying a secret by default"). This builder
 * goes further than that all-or-nothing reveal: it never exposes the raw JSON in any form, matching
 * IM's own `credentialFields[kind].secret` manifest precedent of excluding a secret field *by name*,
 * not a partial "keys but not values" reveal (no precedent for that partial shape exists anywhere in
 * this codebase). `hasEnv`/`hasHeaders` below report only whether the map is non-empty
 * (`Object.keys(...).length > 0`) -- the same "a boolean flag, never the value itself" shape IM's
 * `hasCredentials`, SSH's `hasPassword`, and Observability's `otlpAuthConfigured` already
 * established; only key *counts* are touched, never a key name or a value.
 *
 * Traced past the TypeScript types into the Rust side that produces `status.error` specifically
 * (`infrastructure/connection_adapter.rs`'s `RmcpConnectionAdapter::test`, `application/runtime.rs`'s
 * `McpRuntimeError`): a connection failure's raw upstream diagnostic (protocol/HTTP response detail,
 * spawn error text) is captured internally as `McpRuntimeError`'s own `diagnostic` field, but its
 * `Display` impl -- the only thing `error.to_string()` (the sole producer of
 * `ConnectionOutcome::Failed.error`, and from there `ServerStatus.error`) ever reads -- returns only
 * `code.safe_message()`, a fixed string hardcoded per error code, and discards the diagnostic
 * entirely. `application/runtime_tests.rs`'s own
 * `runtime_error_code_and_display_ignore_upstream_diagnostic_text` proves exactly this property with
 * a realistic secret-shaped diagnostic (`"Authorization: Bearer upstream-secret"`). So `status.error`
 * is provably safe the same way Extensions' `lastError` and Plugins' `statusReason` are (looks free
 * text, isn't) -- but it is also fully redundant with `status.errorCode` (always the same static
 * string for a given code, no variable content), so it is left out anyway in favor of the bounded,
 * pasteable `errorCode` alone, matching CLI Management/LSP's own precedent of keeping remediation
 * codes raw rather than duplicating them as translated prose.
 *
 * `description` is excluded as free text the user typed -- the same "user-authored description"
 * example `diagnostic-field.ts`'s own doc comment already names. `projectPath` is excluded for two
 * independent reasons: it is not rendered anywhere on this page today (matching SSH/Local Media's own
 * "would invent a new surface" precedent for unrendered fields), and even if it were, it is a real
 * absolute local filesystem path -- the same private-external-identifier shape SSH's own excluded
 * `defaultPath`/`keyPath` already are. `tools` (the advertised MCP tool names/descriptions) is
 * excluded as the remote server's own operational content, not this client's health/config status --
 * no other page's builder dumps a remote system's own advertised content, and nothing rules out a
 * misconfigured or malicious server advertising a tool name/description containing anything.
 *
 * `command`/`args`/`url` are kept: unlike `env`/`headers`, none of the three has ever been masked by
 * this page's own form, and all three are already fully visible, unmasked, combined into one string
 * on this exact card today (`endpoint` in `mcp-server-card.tsx`) -- the same "match what is already
 * shown on screen" ground CLI Management's own `executablePath` inclusion already used. Each is
 * `null` when the server's `transportType` doesn't apply to it (stdio has no `url`; sse/streamable_http
 * has no `command`/`args`) -- an applicable-but-currently-inapplicable-for-this-instance field, the
 * same shape CLI Management's own `installation` field already has, not a page-wide inapplicable
 * category like IM's omitted "version"/"safe path". `args` collapses an empty list to unavailable
 * rather than an empty line, matching Extensions' own `packages` field.
 *
 * Field labels reuse `McpServerForm`'s own existing generic labels wherever the concept is identical
 * (`mcp.form.name`/`.scope`/`.transport`/`.command`/`.args`/`.url`/`.enabled`), matching SSH's own
 * precedent of reusing `sshConnections.fields.authMode` rather than duplicating an equivalent label
 * under a new key. `active` is labeled with the page's own established "Enabled" language
 * (`mcp.form.enabled`) rather than a literal "Active" label invented fresh, since every other user-
 * facing surface on this page already talks about this server in Enable/Disable terms.
 */
export function buildMcpDiagnosticFields(
  server: McpServerConfig,
  status: McpServerStatus | undefined,
  t: TFunction,
): DiagnosticField[] {
  const isStdio = server.transportType === "stdio";
  const args = server.args ?? [];
  const hasEnv = Boolean(server.env && Object.keys(server.env).length > 0);
  const hasHeaders = Boolean(server.headers && Object.keys(server.headers).length > 0);

  return [
    { label: t("mcp.form.name"), value: server.name },
    { label: t("mcp.form.transport"), value: server.transportType },
    { label: t("mcp.form.scope"), value: server.scope },
    { label: t("mcp.form.enabled"), value: String(server.active) },
    { label: t("mcp.form.command"), value: isStdio ? server.command ?? null : null },
    { label: t("mcp.form.args"), value: isStdio && args.length > 0 ? args.join(" ") : null },
    { label: t("mcp.diagnostics.field.hasEnv"), value: isStdio ? String(hasEnv) : null },
    { label: t("mcp.form.url"), value: !isStdio ? server.url ?? null : null },
    { label: t("mcp.diagnostics.field.hasHeaders"), value: !isStdio ? String(hasHeaders) : null },
    { label: t("mcp.diagnostics.field.connectionStatus"), value: status?.connectionStatus ?? null },
    { label: t("mcp.diagnostics.field.errorCode"), value: status?.errorCode ?? null },
    { label: t("mcp.diagnostics.field.durationMs"), value: status?.durationMs != null ? String(status.durationMs) : null },
    { label: t("mcp.diagnostics.field.lastConnected"), value: status?.lastConnected ?? null },
  ];
}
