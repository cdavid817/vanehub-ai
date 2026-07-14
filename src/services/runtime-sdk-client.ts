import type { SdkService } from "./sdk-service";
import { tauriSdkClient } from "./tauri-sdk-client";
import { webSdkClient } from "./web-sdk-client";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function createSdkService(): SdkService {
  if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
    return tauriSdkClient;
  }

  return webSdkClient;
}

export const sdkService = createSdkService();
