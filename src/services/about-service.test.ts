import { describe, expect, it } from "vitest";
import { aboutCurrentVersion, checkAboutUpdates, compareVersions } from "./about-service";

/**
 * A release one patch ahead of whatever this build is.
 *
 * Spelling the newer version as a literal ties the test to the shipped version: the moment the
 * project bumped past it, the "newer release" fixture became an older one and the test failed on
 * the release commit rather than on any change to update detection.
 */
const nextPatchVersion = (() => {
  const [major, minor, patch] = aboutCurrentVersion.split(".").map(Number);
  return `${major}.${minor}.${patch + 1}`;
})();

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
          html_url: `https://github.com/cdavid817/vanehub-ai/releases/tag/v${nextPatchVersion}`,
          name: `VaneHub AI v${nextPatchVersion}`,
          tag_name: `v${nextPatchVersion}`,
        }),
        { status: 200 },
      );

    const result = await checkAboutUpdates(fetchImpl);

    expect(result.updateAvailable).toBe(true);
    expect(result.latestVersion).toBe(nextPatchVersion);
    expect(result.releaseNotes).toBe("Release notes");
  });

  it("reports failed update checks", async () => {
    const fetchImpl: typeof fetch = async () => new Response("rate limited", { status: 403 });

    await expect(checkAboutUpdates(fetchImpl)).rejects.toThrow("HTTP 403");
  });
});
