// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { runConfigFixture } from "../../test/run-config-fixture";
import { ChatInputBox } from "./ChatInputBox";

const participants = [
  { mention: "Reviewer", roleName: "Reviewer", agentName: "Codex", modelFamily: "openai" as const, avatar: "R" },
  { mention: "Architect", roleName: "Architect", agentName: "Claude", modelFamily: "anthropic" as const, avatar: "A" },
];

function renderComposer(overrides: Partial<Parameters<typeof ChatInputBox>[0]> = {}) {
  const onAddFileReference = vi.fn();
  const onChange = vi.fn();
  const onSubmit = vi.fn();
  const view = renderWithAppProviders(
    <ChatInputBox
      agents={[]} availableModes={["inherit"]} availableModels={[]} availableReasoning={["low"]}
      runConfig={runConfigFixture({ agentId: "codex-cli", interactionMode: "cli", executionMode: "inherit", streaming: true, thinking: false, longContext: false })}
      fileReferenceCandidates={[{ name: "README.md", path: "README.md" }]}
      fileReferences={[]} isStreaming={false} participantMentions={participants} value="@"
      onAddFileReference={onAddFileReference} onChange={onChange} onClear={() => undefined}
      onRemoveFileReference={() => undefined} onStop={() => undefined} onSubmit={onSubmit}
      {...overrides}
    />,
  );
  return { onAddFileReference, onChange, onSubmit, ...view };
}

describe("composer mention keyboard navigation", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("starts unselected and treats either first arrow as the first unified option", () => {
    renderComposer();
    const editor = screen.getByRole("textbox");
    const options = screen.getAllByRole("option");
    expect(options.every((option) => option.getAttribute("aria-selected") === "false")).toBe(true);

    fireEvent.keyDown(editor, { key: "ArrowUp" });
    expect(options[0].getAttribute("aria-selected")).toBe("true");
    expect(editor.getAttribute("aria-activedescendant")).toBe(options[0].id);
    expect(options[0].getAttribute("data-active")).toBe("true");
  });

  it("moves through participant and file results without wrapping", () => {
    renderComposer();
    const editor = screen.getByRole("textbox");
    const options = screen.getAllByRole("option");
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    expect(options[2].getAttribute("aria-selected")).toBe("true");
    fireEvent.keyDown(editor, { key: "ArrowUp" });
    expect(options[1].getAttribute("aria-selected")).toBe("true");
  });

  it("activates the selected result through the existing participant and file paths", () => {
    const participant = renderComposer();
    const editor = screen.getByRole("textbox");
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    fireEvent.keyDown(editor, { key: "Enter" });
    expect(participant.onChange).toHaveBeenCalledWith("@Reviewer ");
    participant.unmount();

    const file = renderComposer({ participantMentions: [] });
    const fileEditor = screen.getByRole("textbox");
    fireEvent.keyDown(fileEditor, { key: "ArrowUp" });
    fireEvent.keyDown(fileEditor, { key: "Enter" });
    expect(file.onAddFileReference).toHaveBeenCalledWith({ name: "README.md", path: "README.md" }, {});
    expect(file.onChange).toHaveBeenCalledWith("@README.md ");
  });

  it("clears selection with Escape and preserves normal Enter submission", () => {
    const { onChange, onSubmit } = renderComposer();
    const editor = screen.getByRole("textbox");
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    fireEvent.keyDown(editor, { key: "Escape" });
    expect(editor.hasAttribute("aria-activedescendant")).toBe(false);
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.keyDown(editor, { key: "Enter" });
    expect(onSubmit).toHaveBeenCalledOnce();
  });

  it("does not intercept Shift+Enter or IME composition", () => {
    const { onChange, onSubmit } = renderComposer();
    const editor = screen.getByRole("textbox");
    fireEvent.keyDown(editor, { key: "ArrowDown", isComposing: true });
    expect(editor.hasAttribute("aria-activedescendant")).toBe(false);
    fireEvent.keyDown(editor, { key: "ArrowDown" });
    fireEvent.keyDown(editor, { key: "Enter", shiftKey: true });
    expect(onChange).not.toHaveBeenCalled();
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
