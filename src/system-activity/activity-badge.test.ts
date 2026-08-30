import { describe, expect, it } from "vitest";
import { formatActivityUnreadBadge } from "./activity-badge";

describe("formatActivityUnreadBadge", () => {
  it("keeps exact retained counts through 99 and bounds larger displays", () => {
    expect(formatActivityUnreadBadge(0)).toBe("0");
    expect(formatActivityUnreadBadge(99)).toBe("99");
    expect(formatActivityUnreadBadge(100)).toBe("99+");
    expect(formatActivityUnreadBadge(10_000)).toBe("99+");
  });

  it("fails closed for invalid presentation counts", () => {
    expect(formatActivityUnreadBadge(-1)).toBe("0");
    expect(formatActivityUnreadBadge(Number.NaN)).toBe("0");
  });
});
