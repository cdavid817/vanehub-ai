import type { PermissionsService } from "./permissions";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriPermissionsClient } from "./tauri-permissions-client";
import { webPermissionsClient } from "./web-permissions-client";

export function createPermissionsService(): PermissionsService {
  return createRuntimeAdapter({
    tauri: tauriPermissionsClient,
    webMock: webPermissionsClient,
  });
}

export const permissionsService = createPermissionsService();
