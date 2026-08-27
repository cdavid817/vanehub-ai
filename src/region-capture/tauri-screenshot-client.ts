import { invoke } from "@tauri-apps/api/core";

import type { StagedOcrSource } from "../types/local-media";
import type { ScreenshotService } from "./screenshot-service-contract";

export const tauriScreenshotClient: ScreenshotService = {
  selectAndStageScreenshotRegion(input) {
    return invoke<StagedOcrSource | null>("select_and_stage_screenshot_region", { request: input });
  },
  async commitScreenshotSelection(input) {
    await invoke("commit_screenshot_selection", { request: input });
  },
  async cancelScreenshotSelection(input) {
    await invoke("cancel_screenshot_selection", { request: input });
  },
  async cancelActiveScreenshotSelection() {
    await invoke("cancel_active_screenshot_selection");
  },
};
