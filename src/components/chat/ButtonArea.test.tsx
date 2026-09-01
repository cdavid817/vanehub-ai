// @vitest-environment jsdom

import { fireEvent, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig, ModelInfo } from "../../types/chat";
import { ButtonArea } from "./ButtonArea";
import { useRunConfigurationOverrides } from "./hooks/useRunConfigurationOverrides";

const agent: AgentRegistryEntry = {
  id: "codex-cli",
  displayName: "Codex",
  provider: "OpenAI",
  launch: { kind: "cli", executableName: "codex" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: [],
  agentOrigin: "builtin",
};

const model: ModelInfo = {
  id: "gpt-5-5",
  label: "GPT-5.5",
  providerId: "openai",
  supportsReasoning: true,
  maxReasoningDepth: "max",
  supportsLongContext: true,
};

const baseConfig: ChatConfig = {
  agentId: "codex-cli",
  interactionMode: "cli",
  executionMode: "inherit",
  agentPolicy: "standard",
  effectiveExecutionPolicy: "ask",
  providerId: "openai",
  modelId: "gpt-5-5",
  streaming: true,
  thinking: true,
  longContext: false,
};

// A thin harness rather than a hand-mocked `RunConfigurationOverrides`: exercising the real
// hook is what actually proves setOverride/resetOverride/sourceOf wire through the popover
// correctly, not just that ButtonArea calls whatever function object it was handed.
function Harness({ config = baseConfig }: { config?: ChatConfig }) {
  const runConfig = useRunConfigurationOverrides("session-1", config);
  return (
    <ButtonArea
      agents={[agent]}
      availableModes={["inherit", "plan", "execute"]}
      availableModels={[model]}
      availableReasoning={["low", "medium", "high"]}
      canSubmit
      isStreaming={false}
      onStop={() => undefined}
      onSubmit={() => undefined}
      runConfig={runConfig}
      runnerSelector={<div data-testid="stub-runner-selector">runner slot</div>}
    />
  );
}

describe("ButtonArea run configuration (10.15-10.18)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("shows a compact Agent · Model summary with no risk warning for a non-risky policy", () => {
    renderWithAppProviders(<Harness />);
    expect(screen.getByTestId("composer-config-summary").textContent).toBe("Codex · GPT-5.5");
    const policy = screen.getByTestId("effective-execution-policy");
    expect(policy.textContent).toContain("Approval required for risky actions");
    expect(within(policy).queryByText("High risk")).toBeNull();
  });

  it("keeps the compact summary and toolbar testid present even with no toolbar interaction", () => {
    renderWithAppProviders(<Harness />);
    expect(screen.getByTestId("composer-toolbar")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Send" })).toBeTruthy();
  });

  it("surfaces a high-risk warning badge on the closed-state summary when the effective policy allows automatic execution", () => {
    renderWithAppProviders(<Harness config={{ ...baseConfig, agentPolicy: "yolo", effectiveExecutionPolicy: "allow" }} />);
    const policy = screen.getByTestId("effective-execution-policy");
    expect(within(policy).getByText("High risk")).toBeTruthy();
    expect(policy.textContent).toContain("Automatic execution allowed");
    // The popover itself is still closed — the warning is not gated behind opening it.
    expect(screen.queryByTestId("composer-config-popover")).toBeNull();
  });

  it("opens the popover from a closed trigger, includes the runner selector slot, and closes on Escape", async () => {
    const { user } = renderWithAppProviders(<Harness />);
    const trigger = screen.getByTestId("composer-config-trigger");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByTestId("composer-config-popover")).toBeNull();

    await user.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByTestId("composer-config-popover")).toBeTruthy();
    expect(screen.getByTestId("stub-runner-selector")).toBeTruthy();

    fireEvent.keyDown(screen.getByTestId("composer-config-popover"), { key: "Escape" });
    expect(screen.queryByTestId("composer-config-popover")).toBeNull();
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("stages a this-message-only override with provenance, and its own Reset reverts it", async () => {
    const { user } = renderWithAppProviders(<Harness />);
    await user.click(screen.getByTestId("composer-config-trigger"));

    const toggle = screen.getByRole("button", { name: "Long Context" });
    // Every field starts on the profile value, so "From profile" appears once per field —
    // scope to this field's own ConfigField wrapper rather than a page-wide text search.
    const field = within(toggle.closest("div.flex.flex-col") as HTMLElement);
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
    expect(field.getByText("From profile")).toBeTruthy();
    expect(field.queryByRole("button", { name: "Reset Long Context to profile" })).toBeNull();

    await user.click(toggle);
    expect(toggle.getAttribute("aria-pressed")).toBe("true");
    expect(field.getByText("This message only")).toBeTruthy();

    const resetField = field.getByRole("button", { name: "Reset Long Context to profile" });
    await user.click(resetField);
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
    expect(field.getByText("From profile")).toBeTruthy();
    expect(field.queryByRole("button", { name: "Reset Long Context to profile" })).toBeNull();
  });

  it("only offers Reset all once an override exists, and it clears every staged override at once", async () => {
    const { user } = renderWithAppProviders(<Harness />);
    await user.click(screen.getByTestId("composer-config-trigger"));

    expect(screen.queryByRole("button", { name: "Reset all to profile" })).toBeNull();

    await user.click(screen.getByRole("button", { name: "Long Context" }));
    await user.click(screen.getByRole("button", { name: "Thinking" }));
    expect(screen.getAllByText("This message only")).toHaveLength(2);

    await user.click(screen.getByRole("button", { name: "Reset all to profile" }));
    expect(screen.queryByText("This message only")).toBeNull();
    expect(screen.queryByRole("button", { name: "Reset all to profile" })).toBeNull();
  });
});
