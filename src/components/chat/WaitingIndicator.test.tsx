// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { WaitingIndicator } from "./WaitingIndicator";

/** Mirrors `projects.test.tsx`'s own `stubMatchMedia` -- keyed by query so only the
 *  reduced-motion query this component reads is affected, not every `matchMedia` caller. */
function stubMatchMedia(matches: (query: string) => boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: (query: string): MediaQueryList => ({
      matches: matches(query),
      media: query,
      onchange: null,
      addEventListener: () => undefined,
      addListener: () => undefined,
      dispatchEvent: () => false,
      removeEventListener: () => undefined,
      removeListener: () => undefined,
    }),
  });
}

const defaultMatchMedia = window.matchMedia;

describe("WaitingIndicator", () => {
  beforeAll(async () => activateAppLanguage("en"));

  afterEach(() => {
    Object.defineProperty(window, "matchMedia", { configurable: true, value: defaultMatchMedia });
  });

  it("spins the loading icon by default", () => {
    stubMatchMedia(() => false);
    const { container } = render(<WaitingIndicator />);
    expect(container.querySelector("svg")?.getAttribute("class")).toContain("animate-spin");
  });

  it("does not animate the loading icon when the reader prefers reduced motion", () => {
    stubMatchMedia((query) => query === "(prefers-reduced-motion: reduce)");
    const { container } = render(<WaitingIndicator />);
    const icon = container.querySelector("svg");
    expect(icon?.getAttribute("class")).not.toContain("animate-spin");
    // Still renders the same icon and label -- reduced motion drops the animation, not the status.
    expect(screen.getByText("Waiting for response")).toBeTruthy();
  });
});
