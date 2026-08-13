import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriWorkBoardClient } from "./tauri-work-board-client";
import { webWorkBoardClient } from "./web-work-board-client";
import type { WorkBoardService } from "./work-board-service";

export function createWorkBoardService(): WorkBoardService {
  return createRuntimeAdapter({ tauri: tauriWorkBoardClient, webMock: webWorkBoardClient });
}

export const workBoardService = createWorkBoardService();
