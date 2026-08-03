// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { getOnePieceProviderPresets } from "../config/onepiece-provider-presets";
import { hasBundledProviderIcon, ProviderBrandIcon } from "./provider-brand-icon";

describe("ProviderBrandIcon", () => {
  it("renders a bundled vector badge for a known provider", () => {
    render(<ProviderBrandIcon iconKey="openai" label="OpenAI" />);
    const icon = screen.getByRole("img", { name: "OpenAI" });
    expect(icon.querySelector("img")?.getAttribute("src")).toContain("image/svg+xml");
    expect(icon.className).toContain("bg-white");
    expect(icon.className).not.toMatch(/emerald|lime|green/);
  });

  it("renders the initials fallback for an unknown provider", () => {
    render(<ProviderBrandIcon iconKey="unknown" label="Unknown" />);
    expect(screen.getByRole("img", { name: "Unknown" }).textContent).toBe("AI");
  });

  it("bundles an unmodified mark for every provider in the fixed directory", () => {
    const missing = getOnePieceProviderPresets()
      .filter((provider) => !hasBundledProviderIcon(provider.iconKey))
      .map((provider) => provider.id);

    expect(missing).toEqual([]);
  });
});
