import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// jsdom implements no scroll layout at all, so it never defines `scrollIntoView` — any component
// that calls it (e.g. session-row-list.tsx's scroll-anchor effect) throws "is not a function" in
// every test that mounts it, not just ones that call the method directly.
if (typeof Element !== "undefined" && !Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => undefined;
}

// jsdom implements no ResizeObserver either -- any component using a container-width check (e.g.
// use-table-compact-mode.ts, mission-control-section-nav.tsx) throws "ResizeObserver is not
// defined" the instant it mounts in a test that never stubs one locally, not just tests that assert
// on resize behavior. A no-op default here means those components mount safely everywhere by
// default; their own tests still override it locally (via vi.stubGlobal) when they need the
// callback to actually fire.
if (typeof globalThis !== "undefined" && typeof globalThis.ResizeObserver === "undefined") {
  class NoopResizeObserver {
    observe() { /* no-op: jsdom has no real layout to observe */ }
    unobserve() { /* no-op */ }
    disconnect() { /* no-op */ }
  }
  globalThis.ResizeObserver = NoopResizeObserver as unknown as typeof ResizeObserver;
}

if (typeof window !== "undefined" && !window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: (query: string): MediaQueryList => ({
      matches: false,
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

afterEach(() => {
  if (typeof document !== "undefined") cleanup();
});
