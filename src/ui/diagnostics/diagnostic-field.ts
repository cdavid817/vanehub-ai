/**
 * spec.md "Copyable safe settings diagnostics": a bounded, plain-text-safe field for a page's
 * own copy-diagnostics summary. `value: null` is a field with no reliable source right now --
 * rendered as an explicit "unavailable" marker, never guessed at or left silently absent (a
 * missing key would read as "this page has nothing to say," which is a different, false claim).
 *
 * Deliberately excludes anything free-text or unbounded (a raw error message, a log excerpt, a
 * user-authored description) -- every field a page builds here must already be a version string,
 * a value from a backend-pinned enum/reason-code union, a stable id, a path already shown
 * on-screen, or a timestamp. That constraint lives in each page's own builder, not in this type,
 * which cannot itself tell a bounded string from an unbounded one.
 */
export interface DiagnosticField {
  label: string;
  value: string | null;
}

export function formatDiagnosticSummary(fields: readonly DiagnosticField[], unavailableLabel: string): string {
  return fields.map((field) => `${field.label}: ${field.value ?? unavailableLabel}`).join("\n");
}
