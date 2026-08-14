// @vitest-environment jsdom

import { beforeAll, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { SlashCommand } from "../../services/slash-commands/types";
import { ChatInputBox } from "./ChatInputBox";

const command = (name: string): SlashCommand => ({
  name, category: "info", appliesTo: () => true, run: async () => ({ kind: "handled" }),
});

function renderBox(overrides: Partial<Parameters<typeof ChatInputBox>[0]> = {}) {
  return renderWithAppProviders(
    <ChatInputBox
      agents={[]} availableModes={["inherit"]} availableModels={[]} availableReasoning={["low"]}
      config={{ agentId: "onepiece", interactionMode: "api", executionMode: "inherit", streaming: true, thinking: false, longContext: false }}
      fileReferenceCandidates={[]} fileReferences={[]} isStreaming={false} value=""
      onAddFileReference={() => undefined} onChange={() => undefined} onClear={() => undefined}
      onConfigAgentChange={() => undefined} onConfigLongContextChange={() => undefined}
      onConfigModeChange={() => undefined} onConfigModelChange={() => undefined}
      onConfigProviderChange={() => undefined} onConfigReasoningChange={() => undefined}
      onConfigStreamingChange={() => undefined} onConfigThinkingChange={() => undefined}
      onRemoveFileReference={() => undefined} onStop={() => undefined} onSubmit={() => undefined}
      {...overrides}
    />,
  );
}

describe("ChatInputBox slash command surfaces", () => {
  // The app defaults to zh-CN; pin English so the assertions below check real copy.
  beforeAll(async () => activateAppLanguage("en"));

  it("renders neither surface by default", () => {
    renderBox();
    expect(screen.queryByTestId("slash-command-output")).toBeNull();
    expect(screen.queryByText("Commands")).toBeNull();
  });

  it("renders the completion dropdown from suggestions", () => {
    renderBox({ slashCommandSuggestions: [command("status")] });
    expect(screen.getByRole("button", { name: /\/status/ })).not.toBeNull();
  });

  it("reports the selected command", async () => {
    const onSelectSlashCommand = vi.fn();
    const { user } = renderBox({ slashCommandSuggestions: [command("usage")], onSelectSlashCommand });
    await user.click(screen.getByRole("button", { name: /\/usage/ }));
    expect(onSelectSlashCommand).toHaveBeenCalledWith("usage");
  });

  it("renders command output and forwards dismissal", async () => {
    const onDismissSlashCommandOutput = vi.fn();
    const { user } = renderBox({
      onDismissSlashCommandOutput,
      slashCommandOutput: { titleKey: "slash.output.applied", tone: "info", messages: [] },
    });
    expect(screen.getByTestId("slash-command-output")).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "Dismiss command output" }));
    expect(onDismissSlashCommandOutput).toHaveBeenCalled();
  });
});
