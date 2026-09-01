import type { RunConfigurationOverrides } from "../components/chat/hooks/useRunConfigurationOverrides";
import type { ChatConfig } from "../types/chat";

/**
 * A no-op `RunConfigurationOverrides` around a fixed profile config, for the many `ChatInputBox`
 * test suites that only care about rendering a given effective config and never exercise
 * override staging itself (that behavior is covered directly in `ButtonArea.test.tsx`).
 */
export function runConfigFixture(config: ChatConfig): RunConfigurationOverrides {
  return {
    clearAfterSend: () => undefined,
    effectiveConfig: config,
    hasOverrides: false,
    resetAllOverrides: () => undefined,
    resetOverride: () => undefined,
    setOverride: () => undefined,
    sourceOf: () => "profile",
  };
}
