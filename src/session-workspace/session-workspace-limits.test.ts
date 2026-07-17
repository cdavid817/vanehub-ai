import { describe, expect, it } from "vitest";
import { sessionWorkspaceLimits } from "./session-workspace-limits";

describe("session workspace first-version limits", () => {
  it("keeps local reads and aggregations explicitly bounded", () => {
    expect(sessionWorkspaceLimits).toEqual({
      directoryEntries: 500,
      documentDepth: 6,
      documents: 300,
      fileBytes: 1_048_576,
      diffBytes: 2_097_152,
      logPage: 200,
      messageHistory: 1000,
    });
  });
});

