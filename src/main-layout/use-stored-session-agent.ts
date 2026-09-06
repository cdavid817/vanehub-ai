import { useEffect, useRef } from "react";
import { defaultSessionAgent, previousSessionAgentStorageKey } from "./create-session-agents";
import type { AgentRegistryEntry } from "../types/agent";

/**
 * Applies the remembered Agent choice once the Agent list is actually available.
 *
 * Separate from the dialog's reset effect, and deliberately not part of it. The dialog can open
 * before the Agent query resolves, and a reset running against an empty list picks nothing — the
 * selection then falls through to whichever Agent sorts first, so a user whose last session used
 * OnePiece silently gets a CLI agent instead.
 *
 * Re-running the whole reset when the list lands would fix that by discarding anything typed in
 * the meantime. This applies only the Agent, and only once per opening, so a later refetch of the
 * same list cannot move a selection the user has since changed by hand.
 */
export function useStoredSessionAgent({
  availableAgents,
  onApply,
  open,
}: {
  availableAgents: AgentRegistryEntry[];
  onApply: (agent: AgentRegistryEntry) => void;
  open: boolean;
}) {
  const applied = useRef(false);
  // Read through a ref so a new inline callback on each render cannot re-trigger the effect.
  const apply = useRef(onApply);
  apply.current = onApply;

  useEffect(() => {
    if (!open) {
      applied.current = false;
      return;
    }
    if (applied.current || availableAgents.length === 0) return;
    applied.current = true;
    const stored = defaultSessionAgent(
      availableAgents,
      window.localStorage.getItem(previousSessionAgentStorageKey),
    );
    if (stored) apply.current(stored);
  }, [availableAgents, open]);
}
