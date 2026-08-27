// @vitest-environment jsdom

import { describe, expect, it, vi } from "vitest";
import { watchSurfaceReadiness } from "./bootstrap-readiness";

async function flushMutationObserver() {
  await Promise.resolve();
}

describe("surface bootstrap readiness", () => {
  it("waits until the application root contains a rendered surface", async () => {
    const root = document.createElement("div");
    const onReady = vi.fn();

    watchSurfaceReadiness(root, onReady);
    await flushMutationObserver();
    expect(onReady).not.toHaveBeenCalled();

    root.append(document.createElement("main"));
    await flushMutationObserver();
    expect(onReady).toHaveBeenCalledOnce();
  });

  it("can be stopped before a bootstrap failure surface is rendered", async () => {
    const root = document.createElement("div");
    const onReady = vi.fn();
    const stop = watchSurfaceReadiness(root, onReady);

    stop();
    root.append(document.createElement("main"));
    await flushMutationObserver();
    expect(onReady).not.toHaveBeenCalled();
  });
});
