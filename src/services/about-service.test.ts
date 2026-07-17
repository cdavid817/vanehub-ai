import { describe, expect, it } from "vitest";
import { checkAboutUpdates, compareVersions } from "./about-service";

describe("about-service", () => {
  it("compares semantic versions with optional v prefix", () => {
    expect(compareVersions("v1.2.0", "1.1.9")).toBe(1);
    expect(compareVersions("0.1.0", "v0.1.0")).toBe(0);
    expect(compareVersions("0.1.0", "0.2.0")).toBe(-1);
  });

  it("detects an available GitHub release update", async () => {
    const fetchImpl: typeof fetch = async () =>
      new Response(
        JSON.stringify({
          body: "Release notes",
          html_url: "https://github.com/cdavid817/vanehub-ai/releases/tag/v0.2.0",
          name: "VaneHub AI v0.2.0",
          tag_name: "v0.2.0",
        }),
        { status: 200 },
      );

    const result = await checkAboutUpdates(fetchImpl);

    expect(result.updateAvailable).toBe(true);
    expect(result.latestVersion).toBe("0.2.0");
    expect(result.releaseNotes).toBe("Release notes");
  });

  it("reports failed update checks", async () => {
    const fetchImpl: typeof fetch = async () => new Response("rate limited", { status: 403 });

    await expect(checkAboutUpdates(fetchImpl)).rejects.toThrow("HTTP 403");
  });
});
