import { describe, expect, it } from "vitest";
import { requireHttpsExternalUrl } from "./external-url";

describe("external URL validation", () => {
  it("accepts and normalizes HTTPS provider links", () => {
    expect(requireHttpsExternalUrl("https://console.anthropic.com/settings/keys")).toBe("https://console.anthropic.com/settings/keys");
  });

  it.each(["http://example.com", "javascript:alert(1)", "not-a-url"])("rejects unsafe external URL %s", (url) => {
    expect(() => requireHttpsExternalUrl(url)).toThrow();
  });
});
