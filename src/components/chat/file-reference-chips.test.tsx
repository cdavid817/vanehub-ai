// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import { runConfigFixture } from "../../test/run-config-fixture";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig, ChatFileReference } from "../../types/chat";
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

const references: ChatFileReference[] = [
  { id: "src/utils.rs:10-20", path: "src/utils.rs", name: "utils.rs", startLine: 10, endLine: 20 },
  { id: "src/utils.rs:50-60", path: "src/utils.rs", name: "utils.rs", startLine: 50, endLine: 60 },
  { id: "src/main.rs", path: "src/main.rs", name: "main.rs" },
];

function renderComposer(onRemoveFileReference: (referenceId: string) => void) {
  return render(
    <ChatInputBox
      agents={[agent]}
      availableModes={["inherit"]}
      availableModels={[]}
      availableReasoning={["medium"]}
      runConfig={runConfigFixture(config)}
      fileReferenceCandidates={[]}
      fileReferences={references}
      isStreaming={false}
      onAddFileReference={vi.fn()}
      onChange={vi.fn()}
      onClear={vi.fn()}
      onRemoveFileReference={onRemoveFileReference}
      onStop={vi.fn()}
      onSubmit={vi.fn()}
      value=""
    />,
  );
}

describe("file reference chips", () => {
  it("labels a ranged reference and leaves a whole-file reference undecorated", () => {
    renderComposer(vi.fn());
    expect(screen.getByText("L10-20")).toBeTruthy();
    expect(screen.getByText("L50-60")).toBeTruthy();
    // Three chips, two markers: the whole-file reference carries none.
    expect(screen.getAllByText(/^L\d/)).toHaveLength(2);
  });

  it("removes only the selected region when one file is referenced twice", () => {
    const onRemove = vi.fn();
    renderComposer(onRemove);
    const removeButtons = screen.getAllByTitle("移除文件引用");
    expect(removeButtons).toHaveLength(3);

    fireEvent.click(removeButtons[0]);
    expect(onRemove).toHaveBeenCalledWith("src/utils.rs:10-20");

    fireEvent.click(removeButtons[1]);
    // Keyed on identity, not path — otherwise removing one region would take both.
    expect(onRemove).toHaveBeenLastCalledWith("src/utils.rs:50-60");
    expect(onRemove).toHaveBeenCalledTimes(2);
  });
});
