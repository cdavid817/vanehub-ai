import { describe, expect, it } from "vitest";
import { formatNotificationTimestamp } from "./notification-center";

describe("formatNotificationTimestamp", () => {
  it("formats notification time in the active locale", () => {
    const timestamp = Date.UTC(2026, 6, 17, 8, 30);

    expect(formatNotificationTimestamp(timestamp, "zh-CN")).toContain("7月");
    expect(formatNotificationTimestamp(timestamp, "en")).toMatch(/Jul/i);
  });
});
