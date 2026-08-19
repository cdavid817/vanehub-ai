import type { ChatConfig } from "../types/chat";
import { readWebMockStorage, writeWebMockStorage } from "./web-mock-storage";

const chatConfigStorageKey = "vanehub.session-chat-config.v1";

// Owned here and never exported. An exported mutable binding re-imported from two modules gives
// two divergent copies of the mock world, which surfaces as one UI panel showing stale data while
// another shows fresh. Callers reach the configs through the accessors below.
let memoryChatConfigs: Record<string, ChatConfig> = {};

// Chat configs carry model and policy selections, never a credential, so browser storage stays
// inside the "Honest Web/mock behavior" prohibition on persisting plaintext secrets.
export function readWebChatConfigs(): Record<string, ChatConfig> {
  return readWebMockStorage(chatConfigStorageKey, memoryChatConfigs);
}

export function writeWebChatConfigs(configs: Record<string, ChatConfig>) {
  memoryChatConfigs = configs;
  writeWebMockStorage(chatConfigStorageKey, configs);
}

/** Deleting a session drops its config, so the delete path is a step rather than a raw write. */
export function deleteWebSessionChatConfig(sessionId: string): void {
  const configs = { ...readWebChatConfigs() };
  delete configs[sessionId];
  writeWebChatConfigs(configs);
}
