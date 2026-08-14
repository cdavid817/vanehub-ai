import { describe, expect, it } from "vitest";
import { webBuiltinToolClient } from "./web-builtin-tool-client";

describe("Web built-in tool adapter", () => {
  it("reports stable simulated unavailability without native probes", async () => {
    const readiness = await webBuiltinToolClient.getBuiltinToolReadiness("onepiece");

    expect(readiness.agentId).toBe("onepiece");
    expect(readiness.capabilities).toHaveLength(8);
    expect(readiness.capabilities.flatMap((item) => item.modes)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          state: "unavailable",
          reasonCode: "desktop_runtime_required",
          simulated: true,
        }),
      ]),
    );
  });

  it("labels readable mock artifacts and refuses native effects", async () => {
    const page = await webBuiltinToolClient.listArtifacts({ sessionId: "session-test" });
    const artifact = page.items[0];

    expect(artifact).toMatchObject({ id: "web-simulated-artifact", simulated: true });
    await expect(webBuiltinToolClient.publishArtifact({
      artifactId: artifact!.id,
      expectedContentHash: artifact!.contentHash,
      acknowledgement: true,
    })).rejects.toThrow("desktop_runtime_required");
    await expect(webBuiltinToolClient.beginBrowserHandoff("operation-1"))
      .rejects.toThrow("desktop_runtime_required");
  });
});
