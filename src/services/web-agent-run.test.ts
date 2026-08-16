import { describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";

describe("Web canonical Run adapter", () => {
  it("keeps bounded query, event, resume, stale-version, and terminal parity", async () => {
    const page = await webAgentClient.listAgentRuns(0, 500, {
      ownerType: "web_demo",
      ownerId: "web-session-open",
    });
    expect(page.limit).toBe(100);
    expect(page.items).toHaveLength(1);
    const paused = page.items[0];
    await expect(webAgentClient.resumeAgentRun(paused.id, paused.version - 1))
      .rejects.toThrow("version conflict");
    const running = await webAgentClient.resumeAgentRun(paused.id, paused.version);
    expect(running.state).toBe("running");
    const cancelled = await webAgentClient.cancelAgentRun(running.id, running.version);
    expect(cancelled.state).toBe("cancelled");
    expect(await webAgentClient.cancelAgentRun(running.id, running.version)).toEqual(cancelled);
    const events = await webAgentClient.listAgentRunEvents(running.id, 0, 500);
    expect(events.at(-1)?.state).toBe("cancelled");
    expect(events.length).toBeLessThanOrEqual(100);
  });
});
