import { describe, expect, it } from "vitest";
import { buildCliParameterPreview } from "../services/cli-parameter-catalog";
import { managedCliAgentIds } from "../types/agent";
import sourceAudit from "./fixtures/cli-parameter-source-audit.json";

describe("CLI parameter catalog audit", () => {
  it("records a current official source for every managed CLI", () => {
    expect(sourceAudit.reviewedAt).toBe("2026-08-23");
    expect(Object.keys(sourceAudit.sources).sort()).toEqual([...managedCliAgentIds].sort());
    expect(Object.values(sourceAudit.sources).every((url) => url.startsWith("https://"))).toBe(true);
    expect(sourceAudit.excludedCategories).toContain("approval-and-permission-policy");
  });

  // The audit was performed against each vendor's published reference *and* the binary installed
  // here, because the two disagreed twice: claude-code hides --advisor from its own --help, and
  // codex-cli 0.149.0 rejects an --ask-for-approval value the registry still listed.
  it("records which binary version each source was cross-checked against", () => {
    expect(Object.keys(sourceAudit.binaries).sort()).toEqual([...managedCliAgentIds].sort());
    expect(
      Object.values(sourceAudit.binaries).every((version) => /^\d+\.\d+\.\d+$/.test(version)),
    ).toBe(true);
  });

  it("applies expanded flags only to their declared launch scopes", () => {
    expect(buildCliParameterPreview("codex-cli", { noAltScreen: true }, "interactive")).toContain("--no-alt-screen");
    expect(buildCliParameterPreview("codex-cli", { noAltScreen: true }, "chat")).not.toContain("--no-alt-screen");
    expect(buildCliParameterPreview("gemini-cli", { debug: true, screenReader: true }, "chat")).toEqual(["--debug"]);
  });
});
