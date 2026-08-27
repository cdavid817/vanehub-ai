import type { StagedOcrSource } from "../types/local-media";

export interface ScreenshotSelection {
  runId: string;
  displayToken: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ScreenshotService {
  selectAndStageScreenshotRegion(input: {
    composerScopeId: string;
  }): Promise<StagedOcrSource | null>;
  commitScreenshotSelection(input: ScreenshotSelection): Promise<void>;
  cancelScreenshotSelection(input: { runId: string }): Promise<void>;
  cancelActiveScreenshotSelection(): Promise<void>;
}
