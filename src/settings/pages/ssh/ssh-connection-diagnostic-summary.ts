import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import type { SshConnection } from "../../../types/ssh-connection";

/**
 * spec.md "Copyable safe settings diagnostics" for one SSH connection. Task 12.19's own audit
 * (tasks.md) already named this page's exact redaction boundary before this file existed: "SSH
 * host/user/keyPath/defaultPath as private external identifiers" must never appear in any page's
 * summary, even though the actual password/key bytes are never client-side to begin with --
 * `SshConnection` has no password/key-content field at all, only the write-only
 * `SaveSshConnectionInput.password` (never read back) and the boolean `hasPassword`, the same
 * flag-not-value boundary IM's own `hasCredentials` already established.
 *
 * `name` is left out too, on a different but compatible ground: it is free text the user typed,
 * the same "user-authored description" category `diagnostic-field.ts`'s own doc comment already
 * excludes -- `id` (a VaneHub-internal opaque identifier, not the remote system's own identity) is
 * included instead as this summary's only "which connection is this" correlating field. `port`,
 * `revision`, and `hostTrust` (the host key's own algorithm/fingerprint/confirmedAt) are left out
 * too: none is rendered anywhere on this page today (matching Local Media's own precedent for
 * `.revision`/`.updatedAt` -- including an unrendered field would invent a new surface, not reuse
 * one), and `hostTrust` in particular pairs a duplicate of the already-excluded host/port with a
 * per-server key fingerprint that, while not itself secret by SSH's own protocol design, is still a
 * strong identifier of a real external system -- genuinely unsure, so left out per this task's own
 * "mark unavailable rather than guess" rule. `lastError` is excluded as unbounded free text (the
 * first example `diagnostic-field.ts` itself names); this page has no separate bounded
 * remediation-code field to report instead, so that category is simply not supported by this page
 * rather than forced onto a raw error string.
 */
export function buildSshConnectionDiagnosticFields(connection: SshConnection, t: TFunction): DiagnosticField[] {
  return [
    { label: t("sshConnections.diagnostics.field.id"), value: connection.id },
    { label: t("sshConnections.fields.authMode"), value: connection.authMode },
    { label: t("sshConnections.diagnostics.field.hasPassword"), value: String(connection.hasPassword) },
    { label: t("sshConnections.diagnostics.field.testStatus"), value: connection.testStatus },
    { label: t("sshConnections.diagnostics.field.createdAt"), value: connection.createdAt },
    { label: t("sshConnections.diagnostics.field.updatedAt"), value: connection.updatedAt },
    { label: t("sshConnections.diagnostics.field.lastConnectedAt"), value: connection.lastConnectedAt },
  ];
}
