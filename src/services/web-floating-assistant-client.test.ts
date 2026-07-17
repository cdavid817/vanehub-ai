import { beforeEach, describe, expect, it } from "vitest";
import { resetWebFloatingAssistantState, webFloatingAssistantClient } from "./web-floating-assistant-client";
import type { FloatingAssistantEvent } from "../types/floating-assistant";

describe("webFloatingAssistantClient", () => {
  beforeEach(() => resetWebFloatingAssistantState());

  it("defaults disabled and normalizes persisted anchors", async () => {
    await expect(webFloatingAssistantClient.getConfig()).resolves.toEqual({ enabled: false, anchor: null });
    await expect(webFloatingAssistantClient.getRuntimeInfo()).resolves.toEqual({
      nativeAvailable: false,
      platform: "unsupported",
    });

    await expect(webFloatingAssistantClient.saveAnchor({ x: Number.NaN, y: 10, monitorName: null }))
      .resolves.toMatchObject({ anchor: null });
  });

  it("emits configuration, surface, main-action, and exit events", async () => {
    const events: FloatingAssistantEvent[] = [];
    const unsubscribe = await webFloatingAssistantClient.subscribeEvents((event) => events.push(event));

    await webFloatingAssistantClient.setEnabled(true);
    await webFloatingAssistantClient.setSurfaceMode("chat");
    await webFloatingAssistantClient.showMainWindow("new-session");
    await webFloatingAssistantClient.exitApplication();

    expect(events.map((event) => event.kind)).toEqual([
      "configuration-changed",
      "surface-changed",
      "main-action",
      "exit-requested",
    ]);
    unsubscribe();
    await webFloatingAssistantClient.setSurfaceMode("collapsed");
    expect(events).toHaveLength(4);
  });
});
