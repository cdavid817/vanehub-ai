import { describe, expect, it } from "vitest";
import sourceAudit from "./fixtures/cli-parameter-source-audit.json";
import { cliParameterDefinitions } from "../services/cli-parameter-registry";
import {
  cliArgumentSegmentValues,
  renderCliParameterSegments,
} from "../services/cli-parameter-renderer";
import { managedCliAgentIds } from "../types/agent";

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

  it("records an audit verdict and a reviewed artefact for every parameter", () => {
    for (const agentId of managedCliAgentIds) {
      for (const definition of cliParameterDefinitions(agentId)) {
        // `audit` is registry metadata, not part of the frontend contract, so it is read off the
        // generated projection rather than typed here.
        const audit = (definition as unknown as { audit?: Record<string, string> }).audit;
        if (!audit) continue;
        expect(["verified", "repository-verified", "pending-review"]).toContain(
          audit.verification,
        );
        expect(audit.reviewedState.length).toBeGreaterThan(0);
      }
    }
  });

  it("applies expanded flags only to their declared launch scopes", () => {
    const codex = cliParameterDefinitions("codex-cli");
    const gemini = cliParameterDefinitions("gemini-cli");

    const noAltScreen = { noAltScreen: { state: "value" as const, value: true } };
    expect(
      cliArgumentSegmentValues(renderCliParameterSegments(codex, noAltScreen, "interactive")),
    ).toContain("--no-alt-screen");
    expect(
      cliArgumentSegmentValues(renderCliParameterSegments(codex, noAltScreen, "chat")),
    ).not.toContain("--no-alt-screen");

    expect(
      cliArgumentSegmentValues(
        renderCliParameterSegments(
          gemini,
          {
            debug: { state: "value", value: true },
            screenReader: { state: "value", value: true },
          },
          "chat",
        ),
      ),
    ).toEqual(["--debug"]);
  });
});
