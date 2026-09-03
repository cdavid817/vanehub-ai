import { CreateSessionDialogContent } from "./create-session-dialog-content";
import { useCreateSessionDraft } from "./use-create-session-draft";
import type { CreateSessionWorkspacePrefill } from "./create-session-workspace-prefill";
import type { AgentRegistryEntry, Session } from "../types/agent";

/**
 * Wires `useCreateSessionDraft` (the draft/validation model, task 11.1) onto
 * `CreateSessionDialogContent`'s existing props. Everything this component used to own directly
 * -- ~26 pieces of `useState`, four effects, the inspect/browse handlers, and submission -- now
 * lives in the hook; this file is left with only the mapping.
 */
export function CreateSessionDialog({
  agents,
  onClose,
  onConfigureOnePiece,
  onCreated,
  open,
  prefillWorkspace,
}: {
  agents: AgentRegistryEntry[];
  onClose: () => void;
  onConfigureOnePiece: () => void;
  onCreated: (session: Session) => void;
  open: boolean;
  /** Task 13.9: forwarded to `useCreateSessionDraft` verbatim -- see its own prop doc comment. */
  prefillWorkspace?: CreateSessionWorkspacePrefill | null;
}) {
  const model = useCreateSessionDraft({ agents, onCreated, open, prefillWorkspace });

  if (!open) return null;

  return <CreateSessionDialogContent model={model} onClose={onClose} onConfigureOnePiece={onConfigureOnePiece} />;
}
