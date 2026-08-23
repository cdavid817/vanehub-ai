import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import sourceAudit from "./fixtures/cli-parameter-source-audit.json";
import { cliParameterDefinitions } from "../services/cli-parameter-registry";
import {
  cliArgumentSegmentValues,
  renderCliParameterSegments,
} from "../services/cli-parameter-renderer";
import { managedCliAgentIds } from "../types/agent";

interface CanonicalAudit {
  evidence: string[];
  reviewedState: string;
}

interface CanonicalAgent {
  agentId: string;
  parameters: { id: string; audit: CanonicalAudit }[];
}

function canonicalAgents(): CanonicalAgent[] {
  const catalog = JSON.parse(
    readFileSync(
      "src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json",
      "utf8",
    ),
  ) as { agents: CanonicalAgent[] };
  return catalog.agents;
}

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

  it("records evidence kinds rather than one verdict for every parameter", () => {
    // A single `verified` cannot distinguish "the vendor documents this" from "the binary's parser
    // accepted it when probed", and those came apart during the audit, so the registry records
    // every kind of evidence it has.
    //
    // Read from the canonical registry, not the generated contract: the generator deliberately
    // drops audit prose, so a loop over the projection would find no `audit` at all and pass while
    // asserting nothing. That is exactly what the previous version of this test did.
    const kinds = [
      "official-reviewed",
      "binary-parser-accepted",
      "live-runtime-verified",
      "repository-verified",
      "pending-review",
    ];
    let audited = 0;
    let probed = 0;
    for (const agent of canonicalAgents()) {
      for (const parameter of agent.parameters) {
        const audit = parameter.audit;
        expect(audit, `${agent.agentId}:${parameter.id} has no audit record`).toBeTruthy();
        expect(audit.evidence.length).toBeGreaterThan(0);
        for (const evidence of audit.evidence) expect(kinds).toContain(evidence);
        // Nothing may claim a live run: the audit probed argument parsing, which is not running.
        expect(audit.evidence).not.toContain("live-runtime-verified");
        if (audit.evidence.includes("pending-review")) expect(audit.evidence).toHaveLength(1);
        expect(audit.reviewedState.length).toBeGreaterThan(0);
        audited += 1;
        if (audit.evidence.includes("binary-parser-accepted")) probed += 1;
      }
    }
    expect(audited).toBeGreaterThan(40);
    expect(probed).toBeGreaterThan(0);
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
