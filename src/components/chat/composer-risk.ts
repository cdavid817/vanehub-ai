import { withEffectiveExecutionPolicy } from "../../services/chat-configuration";
import type { ChatConfig, EffectiveExecutionPolicy } from "../../types/chat";

/**
 * "allow" is the effective-policy value `withEffectiveExecutionPolicy`
 * (`services/chat-configuration.ts`) produces exactly when the agent policy is "trusted" or
 * "yolo" and execution isn't pinned to plan/readonly — the same boundary
 * `agent-policies-page.tsx`'s `requiresConfirmationToAssign` already gates behind an explicit
 * trust-increase confirmation. Reusing that boundary here means the composer's warning and the
 * settings page's confirm dialog agree on what "risky" means, instead of the composer inventing
 * a second risk taxonomy design.md never specified.
 */
export function isHighRiskExecutionPolicy(effectiveExecutionPolicy: EffectiveExecutionPolicy | undefined): boolean {
  return effectiveExecutionPolicy === "allow";
}

/**
 * `ChatConfig.effectiveExecutionPolicy` is set by the server on a profile save (see
 * `useChatConfig`'s debounced persist effect) and never touched again — it goes stale the
 * instant `executionMode` is staged as a this-message-only override, since an override is
 * designed to never reach that round-trip (`useRunConfigurationOverrides` never writes back to
 * the profile). Re-deriving it here, from whatever `executionMode` is currently effective, with
 * the exact formula the server uses, is what keeps the summary and popover honest about an
 * overridden mode instead of silently showing the pre-override policy underneath it.
 */
export function effectivePolicyForDisplay(config: ChatConfig): EffectiveExecutionPolicy | undefined {
  if (!config.agentPolicy) return undefined;
  return withEffectiveExecutionPolicy(config, config.agentPolicy).effectiveExecutionPolicy;
}
