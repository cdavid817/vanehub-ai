import { describe, expect, it } from "vitest";
import { buildSdkVersionOptions, compareSdkVersions, getSdkVersionAction, normalizeSdkVersion } from "./sdk-versioning";

describe("sdk-versioning", () => {
  it("normalizes leading v versions", () => {
    expect(normalizeSdkVersion(" v0.2.81 ")).toBe("0.2.81");
    expect(normalizeSdkVersion("1.2.3")).toBe("1.2.3");
    expect(normalizeSdkVersion("")).toBeUndefined();
  });

  it("compares semantic version cores", () => {
    expect(compareSdkVersions("0.2.81", "0.2.88")).toBeLessThan(0);
    expect(compareSdkVersions("0.2.90", "0.2.88")).toBeGreaterThan(0);
    expect(compareSdkVersions("v0.2.88", "0.2.88")).toBe(0);
  });

  it("derives install update rollback and current actions", () => {
    expect(getSdkVersionAction({ installed: false, requestedVersion: "0.2.88" })).toBe("install");
    expect(getSdkVersionAction({ installed: true, installedVersion: "0.2.81", requestedVersion: "0.2.88" })).toBe("update");
    expect(getSdkVersionAction({ installed: true, installedVersion: "0.2.88", requestedVersion: "0.2.81" })).toBe("rollback");
    expect(getSdkVersionAction({ installed: true, installedVersion: "0.2.88", requestedVersion: "0.2.88" })).toBe("current");
  });

  it("merges unique selectable versions", () => {
    expect(
      buildSdkVersionOptions({
        availableVersions: ["0.2.88", "0.2.81"],
        fallbackVersions: ["0.2.81", "0.2.58"],
        installedVersion: "v0.2.58",
      }),
    ).toEqual(["0.2.88", "0.2.81", "0.2.58"]);
  });
});
