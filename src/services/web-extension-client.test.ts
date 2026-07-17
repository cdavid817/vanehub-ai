import { afterEach, describe, expect, it, vi } from "vitest";
import { webOperationClient } from "./web-operation-client";
import { resetWebExtensionStateForTests, webExtensionClient } from "./web-extension-client";

describe("webExtensionClient", () => {
  afterEach(() => {
    vi.useRealTimers();
    resetWebExtensionStateForTests();
  });

  it("returns the stable built-in catalog in capability order", async () => {
    const overview = await webExtensionClient.getOverview();

    expect(overview.definitions.map((definition) => definition.id)).toEqual([
      "paddleocr",
      "faster-whisper",
      "sherpa-onnx",
    ]);
    expect(overview.definitions.map((definition) => definition.capabilityId)).toEqual(["ocr", "asr", "tts"]);
    expect(overview.environment.nativeOperationsAvailable).toBe(false);
  });

  it("returns a non-mutating desktop-only install preview", async () => {
    const preview = await webExtensionClient.getInstallPreview({ frameworkId: "paddleocr" });
    const overview = await webExtensionClient.getOverview();

    expect(preview.supported).toBe(false);
    expect(preview.inferenceLocalOnly).toBe(true);
    expect(preview.packages).toContain("paddleocr");
    expect(overview.statuses[0].installed).toBe(false);
  });

  it("records native mutations as failed extension operations", async () => {
    vi.useFakeTimers();
    const operation = await webExtensionClient.install({ frameworkId: "faster-whisper" });

    expect(operation.kind).toBe("extension");
    expect(operation.status).toBe("queued");
    await vi.advanceTimersByTimeAsync(950);
    const completed = await webOperationClient.getOperationStatus(operation.id);
    expect(completed.status).toBe("failed");
    expect(completed.error).toContain("Desktop runtime");
  });
});
