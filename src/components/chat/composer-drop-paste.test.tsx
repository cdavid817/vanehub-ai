// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { FILE_REFERENCE_TRANSFER_TYPE } from "../../services/file-reference-transfer";
import type { MentionLineRange } from "../../services/composer-mention";
import { runConfigFixture } from "../../test/run-config-fixture";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import type { FileSearchMatch } from "../../types/session-workspace";
import { ChatInputBox } from "./ChatInputBox";

const config: ChatConfig = {
  agentId: "codex-cli",
  interactionMode: "cli",
  executionMode: "inherit",
  agentPolicy: "readonly",
  effectiveExecutionPolicy: "readonly",
  streaming: true,
  thinking: false,
  longContext: false,
};

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

function transfer(entries: Record<string, string>) {
  return {
    dropEffect: "none",
    effectAllowed: "none",
    types: Object.keys(entries),
    getData: (type: string) => entries[type] ?? "",
    setData: () => undefined,
  };
}

const ourDrag = () => transfer({ [FILE_REFERENCE_TRANSFER_TYPE]: "src/utils.rs", "text/plain": "src/utils.rs" });
const plainText = () => transfer({ "text/plain": "src/utils.rs" });

function renderComposer({ disabled = false, isStreaming = false } = {}) {
  const onAddFileReference = vi.fn<(candidate: FileSearchMatch, range: MentionLineRange) => void>();
  const onChange = vi.fn<(next: string) => void>();
  render(
    <ChatInputBox
      agents={[agent]}
      availableModes={["inherit"]}
      availableModels={[]}
      availableReasoning={["medium"]}
      runConfig={runConfigFixture(config)}
      disabled={disabled}
      fileReferenceCandidates={[]}
      fileReferences={[]}
      isStreaming={isStreaming}
      onAddFileReference={onAddFileReference}
      onChange={onChange}
      onClear={vi.fn()}
      onRemoveFileReference={vi.fn()}
      onStop={vi.fn()}
      onSubmit={vi.fn()}
      sessionId="session-1"
      value=""
    />,
  );
  return { composer: screen.getByTestId("wechat-style-composer"), onAddFileReference, onChange };
}

describe("composer drop and paste", () => {
  it("attaches a whole-file reference from a dropped workspace path", () => {
    const { composer, onAddFileReference } = renderComposer();
    fireEvent.drop(composer, { dataTransfer: ourDrag() });
    expect(onAddFileReference).toHaveBeenCalledWith({ name: "utils.rs", path: "src/utils.rs" }, {});
  });

  it("ignores a drop that is not this application's transfer", () => {
    const { composer, onAddFileReference } = renderComposer();
    fireEvent.drop(composer, { dataTransfer: plainText() });
    expect(onAddFileReference).not.toHaveBeenCalled();
  });

  it("attaches from a paste carrying the transfer type", () => {
    const { composer, onAddFileReference, onChange } = renderComposer();
    fireEvent.paste(composer, { clipboardData: ourDrag() });
    expect(onAddFileReference).toHaveBeenCalledWith({ name: "utils.rs", path: "src/utils.rs" }, {});
    // Attaching replaces insertion; the path must not also land in the draft.
    expect(onChange).not.toHaveBeenCalled();
  });

  it("leaves ordinary paste alone, including text that looks like a path", () => {
    const { composer, onAddFileReference } = renderComposer();
    const event = fireEvent.paste(composer, { clipboardData: plainText() });
    expect(onAddFileReference).not.toHaveBeenCalled();
    // Not prevented, so the textarea inserts it the way it always did.
    expect(event).toBe(true);
  });

  it("shows the drop affordance and keeps it while crossing into a child", () => {
    const { composer } = renderComposer();
    expect(composer.getAttribute("data-drop-target")).toBeNull();

    fireEvent.dragEnter(composer, { dataTransfer: ourDrag() });
    expect(composer.getAttribute("data-drop-target")).toBe("true");

    // Entering a child fires dragleave on the parent; a boolean flag would flicker here.
    const child = composer.querySelector("textarea");
    fireEvent.dragEnter(child as Element, { dataTransfer: ourDrag() });
    fireEvent.dragLeave(composer, { dataTransfer: ourDrag() });
    expect(composer.getAttribute("data-drop-target")).toBe("true");

    fireEvent.dragLeave(composer, { dataTransfer: ourDrag() });
    expect(composer.getAttribute("data-drop-target")).toBeNull();
  });

  it("clears the affordance on drop", () => {
    const { composer } = renderComposer();
    fireEvent.dragEnter(composer, { dataTransfer: ourDrag() });
    fireEvent.drop(composer, { dataTransfer: ourDrag() });
    expect(composer.getAttribute("data-drop-target")).toBeNull();
  });

  it("attaches nothing while the composer is disabled", () => {
    const { composer, onAddFileReference } = renderComposer({ disabled: true });
    fireEvent.drop(composer, { dataTransfer: ourDrag() });
    fireEvent.paste(composer, { clipboardData: ourDrag() });
    expect(onAddFileReference).not.toHaveBeenCalled();
    expect(composer.getAttribute("data-drop-target")).toBeNull();
  });

  it("attaches nothing while a generation is streaming", () => {
    const { composer, onAddFileReference } = renderComposer({ isStreaming: true });
    fireEvent.drop(composer, { dataTransfer: ourDrag() });
    fireEvent.paste(composer, { clipboardData: ourDrag() });
    expect(onAddFileReference).not.toHaveBeenCalled();
  });
});
