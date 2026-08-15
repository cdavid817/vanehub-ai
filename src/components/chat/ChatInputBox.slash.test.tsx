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

  it("keeps the output panel and the completion dropdown both present and interactive together", async () => {
    // Reachable by running /status, leaving its output open, then typing "/" again: output
    // state and the suggestion query are independent, so both props can be non-empty at once.
    const onDismissSlashCommandOutput = vi.fn();
    const onSelectSlashCommand = vi.fn();
    const { user } = renderBox({
      onDismissSlashCommandOutput,
      onSelectSlashCommand,
      slashCommandOutput: { titleKey: "slash.output.applied", tone: "info", messages: [] },
      slashCommandSuggestions: [command("status")],
    });

    const outputPanel = screen.getByTestId("slash-command-output");
    const suggestionButton = screen.getByRole("button", { name: /\/status/ });
    const dismissButton = screen.getByRole("button", { name: "Dismiss command output" });

    // jsdom performs no layout, so this cannot prove the two panels don't visually overlap.
    // It proves the structural fix instead: the output panel no longer carries its own
    // positioning class, and it shares one positioned ancestor with the completion panel
    // rather than each claiming the same coordinates independently.
    expect(outputPanel.className.includes("absolute")).toBe(false);
    const sharedWrapper = outputPanel.closest(".absolute");
    expect(sharedWrapper).not.toBeNull();
    expect(sharedWrapper?.contains(suggestionButton)).toBe(true);

    await user.click(suggestionButton);
    expect(onSelectSlashCommand).toHaveBeenCalledWith("status");
    await user.click(dismissButton);
    expect(onDismissSlashCommandOutput).toHaveBeenCalled();
  });
});
