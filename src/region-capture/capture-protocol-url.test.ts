import { describe, expect, it } from "vitest";

import { captureProtocolUrl } from "./capture-protocol-url";

describe("captureProtocolUrl", () => {
  it("contains only encoded opaque tokens", () => {
    const url = captureProtocolUrl("run value", "display/value");

    expect(url).toContain("run%20value/display%2Fvalue");
    expect(url).not.toContain("\\");
  });
});
