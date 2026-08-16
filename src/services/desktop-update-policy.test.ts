import { describe, expect, it } from "vitest";
import { compareDesktopVersions, defaultUpdateChannel, evaluateUpdatePolicyBatch, isUpdateAdmissible, parseDesktopVersion, validateUpdateManifestCandidate } from "./desktop-update-policy";

describe("desktop update policy", () => {
  it("implements semantic version precedence", () => {
    expect(compareDesktopVersions("1.0.0", "1.0.0-preview.9")).toBe(1);
    expect(compareDesktopVersions("1.0.0-preview.10", "1.0.0-preview.2")).toBe(1);
    expect(compareDesktopVersions("v1.2.3", "1.2.3")).toBe(0);
    expect(parseDesktopVersion("1.02.0")).toBeNull();
  });
  it("blocks prereleases on stable and all downgrades", () => {
    expect(isUpdateAdmissible("1.0.0", "1.1.0-preview.1", "stable")).toBe(false);
    expect(isUpdateAdmissible("1.0.0-preview.1", "1.0.0-preview.2", "preview")).toBe(true);
    expect(isUpdateAdmissible("1.2.0", "1.1.9", "preview")).toBe(false);
    expect(isUpdateAdmissible("1.2.0", "1.2.0", "stable")).toBe(false);
  });
  it("derives safe defaults and rejects insecure manifests", () => {
    expect(defaultUpdateChannel("0.1.0-preview.1")).toBe("preview");
    expect(defaultUpdateChannel("invalid")).toBe("stable");
    const valid = { version: "1.2.0", channel: "stable", signature: "a".repeat(64), url: "https://updates.vanehub.ai/stable.json" };
    expect(validateUpdateManifestCandidate(valid)).toBe(true);
    expect(validateUpdateManifestCandidate({ ...valid, url: "http://attacker.invalid" })).toBe(false);
  });
  it("evaluates a deterministic linear batch", () => {
    const cases = Array.from({ length: 10_000 }, (_, index) => ["1.0.0", `1.0.${index + 1}`, "stable"] as const);
    expect(evaluateUpdatePolicyBatch(cases)).toBe(10_000);
  });
});
