import type { ImService } from "./im-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriImClient } from "./tauri-im-client";
import { webImClient } from "./web-im-client";

export function createImService(): ImService {
  return createRuntimeAdapter({ tauri: tauriImClient, webMock: webImClient });
}

export const imService = createImService();
