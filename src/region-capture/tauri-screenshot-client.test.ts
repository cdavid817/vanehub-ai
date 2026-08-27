import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn(async () => null) }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { tauriScreenshotClient } from "./tauri-screenshot-client";

describe("tauriScreenshotClient", () => {
  beforeEach(() => invoke.mockClear());

  it("keeps screenshot pixels and paths behind native commands", async () => {
    await tauriScreenshotClient.selectAndStageScreenshotRegion({ composerScopeId: "c1" });
    await tauriScreenshotClient.commitScreenshotSelection({
      runId: "run-1",
      displayToken: "display-1",
      x: 10,
      y: 20,
      width: 100,
      height: 80,
    });

    expect(invoke.mock.calls[0]).toEqual([
      "select_and_stage_screenshot_region",
      { request: { composerScopeId: "c1" } },
    ]);
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("png");
    expect(JSON.stringify(invoke.mock.calls)).not.toContain("path");
  });
});
