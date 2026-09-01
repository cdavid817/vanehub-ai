import { useEffect, useMemo, useState } from "react";
import type { ChatConfig } from "../../../types/chat";

export type ConfigFieldSource = "override" | "profile";

export interface RunConfigurationOverrides {
  /** The profile value merged with any active this-message overrides — what should be sent/shown. */
  effectiveConfig: ChatConfig;
  /** Which source produced a field's current effective value, for provenance display (design.md Decision 9). */
  sourceOf: (field: keyof ChatConfig) => ConfigFieldSource;
  /**
   * Stages a this-message-only value. Never reaches `useChatConfig`'s own persisted profile state —
   * that hook's setters (`changeProvider`, `setThinking`, ...) are the "change my default" pathway;
   * this is the separate "just this once" one design.md Decision 9 requires neither auto-write back
   * to the Profile nor get confused with it.
   */
  setOverride: <K extends keyof ChatConfig>(field: K, value: ChatConfig[K]) => void;
  /** Reverts one field back to its persisted profile value. */
  resetOverride: (field: keyof ChatConfig) => void;
  /** Reverts every field at once — the Popover's own "Reset" affordance. */
  resetAllOverrides: () => void;
  hasOverrides: boolean;
  /**
   * A one-shot override does not survive past the message it was staged for — call this once a
   * send actually goes through (not on every keystroke, and not on a failed send: `submit()`'s own
   * existing draft-preservation on failure should extend to "the override the reader picked is
   * still there to retry with", not silently revert to the profile value underneath them).
   */
  clearAfterSend: () => void;
}

/**
 * A separate layer over `useChatConfig`'s output, not a change to that hook: `useChatConfig` owns
 * persistence and session-switch re-initialization already, and duplicating that inside here too
 * would be two places able to disagree about what "the profile value" even is. This hook only ever
 * reads `profileConfig`, never writes back into it.
 */
export function useRunConfigurationOverrides(sessionId: string | null, profileConfig: ChatConfig): RunConfigurationOverrides {
  const [overrides, setOverrides] = useState<Partial<ChatConfig>>({});

  useEffect(() => {
    setOverrides({});
  }, [sessionId]);

  const effectiveConfig = useMemo<ChatConfig>(() => ({ ...profileConfig, ...overrides }), [profileConfig, overrides]);

  function sourceOf(field: keyof ChatConfig): ConfigFieldSource {
    return field in overrides ? "override" : "profile";
  }

  function setOverride<K extends keyof ChatConfig>(field: K, value: ChatConfig[K]) {
    setOverrides((current) => ({ ...current, [field]: value }));
  }

  function resetOverride(field: keyof ChatConfig) {
    setOverrides((current) => {
      if (!(field in current)) return current;
      const next = { ...current };
      delete next[field];
      return next;
    });
  }

  function resetAllOverrides() {
    setOverrides({});
  }

  function clearAfterSend() {
    setOverrides({});
  }

  return {
    clearAfterSend,
    effectiveConfig,
    hasOverrides: Object.keys(overrides).length > 0,
    resetAllOverrides,
    resetOverride,
    setOverride,
    sourceOf,
  };
}
