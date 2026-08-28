import type { ScreenshotService } from "./screenshot-service-contract";

const nativeOnly = () => Promise.reject(new Error("LOCAL_MEDIA_NATIVE_ONLY"));

export const webScreenshotClient: ScreenshotService = {
  async selectAndStageScreenshotRegion() {
    return null;
  },
  commitScreenshotSelection: nativeOnly,
  cancelScreenshotSelection: nativeOnly,
  async cancelActiveScreenshotSelection() {},
};
