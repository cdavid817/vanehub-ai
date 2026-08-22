import type { LocalMediaService } from "./local-media-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriLocalMediaClient } from "./tauri-local-media-client";
import { webLocalMediaClient } from "./web-local-media-client";

export function createLocalMediaService(): LocalMediaService {
  return createRuntimeAdapter({
    tauri: tauriLocalMediaClient,
    webMock: webLocalMediaClient,
  });
}

export const localMediaService = createLocalMediaService();
