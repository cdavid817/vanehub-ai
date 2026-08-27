import { useMemo } from "react";
import { evidenceOperationIdSchema } from "../contracts/session-workspace-evidence-ids";
import { useWorkspaceEvidenceScope } from "./workspace-evidence-scope";

/**
 * Where a relative path can take a reader.
 *
 * Three callbacks that all say the same thing in different destinations: "keep the current scope,
 * add this path, go there". They were written out one at a time in the tab host as each panel
 * needed one, and by the third it was clear they were one idea appearing three times — including
 * the same narrowing check copied twice.
 *
 * Every one is absent until a session is selected. An action that navigated with no session would
 * move the reader to a tab scoped to nothing, which looks like the tab being empty rather than like
 * the action being unavailable.
 */
export interface WorkspacePathNavigation {
  /**
   * Open the execution records for one operation. Absent without a session.
   *
   * The operation is the only correlation an automated finding carries. It is enough: the records
   * tab resolves the run, the trace, and the span from it, so a link that carried them too would
   * be three identifiers this side would have to keep agreeing with a store it cannot read.
   */
  showOperation?: (operationId: string) => void;
  /** Open the Changes tab focused on a path. Absent without a session. */
  showChanges?: (relativePath: string) => void;
  /** Open the execution records for a path. Absent without a session. */
  showEvidence?: (relativePath: string) => void;
  /** Move to the Shell tab, after something has opened one. */
  showShell: () => void;
}

export function useWorkspacePathNavigation(): WorkspacePathNavigation {
  const { activateTab, navigate, scope } = useWorkspaceEvidenceScope();

  return useMemo(() => {
    const sessionId = scope?.sessionId;
    // Narrowed once here rather than asserted at each call site. The scope's session is optional
    // and the two path actions both need it; checking it twice is how the two would eventually
    // disagree about what to do when it is missing.
    const toTab = (tab: "changes" | "traces") =>
      scope && sessionId
        ? (relativePath: string) => navigate({ tab, scope: { ...scope, relativePath, sessionId } })
        : undefined;

    return {
      showChanges: toTab("changes"),
      showEvidence: toTab("traces"),
      showOperation:
        scope && sessionId
          ? (operationId: string) =>
              // Parsed rather than asserted. A finding carries its operation as a plain string, and
              // the brand exists so nothing on this side can invent one; the schema is where a
              // string legitimately becomes an identifier.
              navigate({
                scope: {
                  ...scope,
                  operationId: evidenceOperationIdSchema.parse(operationId),
                  sessionId,
                },
                tab: "terminal",
              })
          : undefined,
      // No scope needed: nothing is being focused, the reader is simply being taken to the Shell
      // that was just created for them.
      showShell: () => activateTab("shell"),
    };
  }, [activateTab, navigate, scope]);
}
