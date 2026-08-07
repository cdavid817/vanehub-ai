import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentWithModelFamily } from "../services/agent-model-family";
import type { SessionSeat } from "../types/agent-seats";
import { SessionSeatAssignment } from "./session-seat-assignment";

function agent(
  id: string,
  displayName: string,
  availabilityState: AgentWithModelFamily["availabilityState"],
  extra: Partial<AgentWithModelFamily> = {},
): AgentWithModelFamily {
  return {
    id,
    displayName,
    provider: "Anthropic",
    availabilityState,
    unavailableReason: null,
    supportedInteractionModes: ["cli"],
    capabilityTags: [],
    managedSdkDependencyId: null,
    modelFamily: "anthropic",
    ...extra,
  } as unknown as AgentWithModelFamily;
}

function render(agents: AgentWithModelFamily[], seats: SessionSeat[]) {
  return renderToStaticMarkup(
    <SessionSeatAssignment
      agents={agents}
      onSeatsChange={() => undefined}
      roles={[]}
      seats={seats}
    />,
  );
}

describe("SessionSeatAssignment", () => {
  beforeAll(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("offers an Agent whose availability is known good", () => {
    const html = render([agent("claude-code", "Claude Code", "available")], [
      { agentId: "claude-code", roleId: null },
    ]);
    expect(html).toContain("Claude Code");
  });

  /**
   * An Agent that has not been probed yet is selectable everywhere else in this dialog. Excluding
   * it only here leaves the seat editor empty with the Create button disabled and nothing saying
   * why — a dead end rather than a rejection.
   */
  it("offers an Agent whose availability is not yet known", () => {
    const html = render([agent("codex-cli", "Codex CLI", "unknown")], [
      { agentId: "codex-cli", roleId: null },
    ]);
    expect(html).toContain("Codex CLI");
  });

  it("offers an Agent whose only obstacle is an uninstalled managed SDK", () => {
    const html = render(
      [
        agent("gemini-cli", "Gemini CLI", "unavailable", {
          managedSdkDependencyId: "gemini-sdk",
          unavailableReason: "Managed SDK dependency 'gemini-sdk' is not installed.",
        }),
      ],
      [{ agentId: "gemini-cli", roleId: null }],
    );
    expect(html).toContain("Gemini CLI");
  });

  it("does not offer an Agent that genuinely cannot run", () => {
    const html = render(
      [
        agent("broken-cli", "Broken CLI", "unavailable", {
          unavailableReason: "Executable not found.",
        }),
      ],
      [{ agentId: "", roleId: null }],
    );
    expect(html).not.toContain("Broken CLI");
  });
});
