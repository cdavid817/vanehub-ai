import type { LocalMediaService } from "./local-media-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriLocalMediaClient } from "./tauri-local-media-client";
import { webLocalMediaClient } from "./web-local-media-client";
import { createDeterministicFakeLocalMediaService } from "../testing/local-media-e2e-fake";

export function createLocalMediaService(): LocalMediaService {
  // A build-time constant, not a runtime switch. Vite folds this to `false` for every build that
  // does not set the variable, and rollup then drops both the branch and the fake module, so a
  // shipped bundle has no fake to activate -- no URL parameter, no storage key, no window global,
  // no command and no settings toggle can reach one that is not in the file.
  if (import.meta.env.VITE_LOCAL_MEDIA_FAKE === "1") {
    return createDeterministicFakeLocalMediaService();
  }
  return createRuntimeAdapter({
    tauri: tauriLocalMediaClient,
    webMock: webLocalMediaClient,
  });
}

export const localMediaService = createLocalMediaService();
