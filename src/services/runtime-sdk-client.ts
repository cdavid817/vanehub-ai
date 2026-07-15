import type { SdkService } from "./sdk-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriSdkClient } from "./tauri-sdk-client";
import { webSdkClient } from "./web-sdk-client";

export function createSdkService(): SdkService {
  return createRuntimeAdapter({
    tauri: tauriSdkClient,
    webMock: webSdkClient,
  });
}

export const sdkService = createSdkService();
