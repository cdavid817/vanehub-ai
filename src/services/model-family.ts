import type { ProviderEndpointType } from "../types/agent";

/**
 * A normalized model family. `AgentRegistryEntry.provider` holds free-form display text such as
 * `"OpenAI"`, so cross-family checks must go through this rather than comparing those strings.
 */
export type ModelFamily = "anthropic" | "openai" | "google" | "unknown";

export interface ModelFamilyInput {
  id: string;
  provider: string;
  endpointType?: ProviderEndpointType | null;
}

/** Built-in agents are keyed by stable id, which cannot drift the way display text can. */
const familyByAgentId: Record<string, ModelFamily> = {
  "claude-code": "anthropic",
  "codex-cli": "openai",
  "gemini-cli": "google",
  // Antigravity speaks Google's own CodeAssist surface and serves Google models, so its family is
  // fixed the way Gemini's is rather than user-configurable like OpenCode's.
  "antigravity-cli": "google",
  // OpenCode drives whichever model the user configured, so it has no fixed family. Claiming one
  // would make a cross-family reviewer check act on a false premise.
  opencode: "unknown",
};

const familyByProviderText: Record<string, ModelFamily> = {
  anthropic: "anthropic",
  claude: "anthropic",
  openai: "openai",
  azureopenai: "openai",
  google: "google",
  gemini: "google",
  googleai: "google",
};

const familyByEndpointType: Record<ProviderEndpointType, ModelFamily> = {
  "anthropic-messages": "anthropic",
  "openai-chat-completions": "openai",
  "openai-responses": "openai",
};

export function normalizeModelFamily(input: ModelFamilyInput): ModelFamily {
  const byId = familyByAgentId[input.id];
  if (byId) return byId;

  const provider = input.provider.trim().toLowerCase().replace(/[\s_-]/g, "");
  const byProvider = familyByProviderText[provider];
  if (byProvider) return byProvider;

  if (input.endpointType) return familyByEndpointType[input.endpointType] ?? "unknown";
  return "unknown";
}

/**
 * Two unknown families are NOT the same family. Treating them as equal would wrongly reject an
 * unknown pair when recommending a cross-family reviewer, which is the opposite of the intent.
 */
export function isSameFamily(left: ModelFamily, right: ModelFamily): boolean {
  if (left === "unknown" || right === "unknown") return false;
  return left === right;
}
