import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("floating assistant theme contract", () => {
  it("uses semantic tokens without theme-name branches or inline styles", () => {
    const surface = readFileSync(new URL("./floating-assistant-app.tsx", import.meta.url), "utf8");
    const settings = readFileSync(
      new URL("../settings/pages/floating-assistant-settings-section.tsx", import.meta.url),
      "utf8",
    );
    const combined = `${surface}\n${settings}`;

    expect(combined).not.toContain("data-theme");
    expect(combined).not.toContain('style={{');
    expect(combined).not.toMatch(/theme\s*===\s*["'](?:minimal|futuristic)/);
    expect(combined).toContain("bg-primary");
    expect(combined).toContain("border-border");
  });
});
