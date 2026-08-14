// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { MentionLineRange } from "../../services/composer-mention";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import type { FileSearchMatch } from "../../types/session-workspace";

const { readSessionFile } = vi.hoisted(() => ({ readSessionFile: vi.fn() }));
vi.mock("../../services/runtime-agent-client", () => ({ agentService: { readSessionFile } }));
vi.mock("../measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>{items.map((item, index) => renderItem(item, index) as never)}</div>
  ),
}));

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

function renderComposer(value: string, sessionId = "session-1") {
  const onAddFileReference = vi.fn<(candidate: FileSearchMatch, range: MentionLineRange) => void>();
  const onChange = vi.fn<(next: string) => void>();
  const element = (activeSession: string) => (
    <ChatInputBox
      agents={[agent]}
      availableModes={["inherit"]}
      availableModels={[]}
      availableReasoning={["medium"]}
      config={config}
      fileReferenceCandidates={[{ name: "utils.rs", path: "src/utils.rs" }]}
      fileReferences={[]}
      isStreaming={false}
      onAddFileReference={onAddFileReference}
      onChange={onChange}
      onClear={vi.fn()}
      onConfigAgentChange={vi.fn()}
      onConfigLongContextChange={vi.fn()}
      onConfigModeChange={vi.fn()}
      onConfigModelChange={vi.fn()}
      onConfigProviderChange={vi.fn()}
      onConfigReasoningChange={vi.fn()}
      onConfigStreamingChange={vi.fn()}
      onConfigThinkingChange={vi.fn()}
      onRemoveFileReference={vi.fn()}
      onStop={vi.fn()}
      onSubmit={vi.fn()}
      sessionId={activeSession}
      value={value}
    />
  );
  const view = render(element(sessionId));
  return {
    onAddFileReference,
    onChange,
    switchSession: (next: string) => view.rerender(element(next)),
  };
}

describe("composer preview trigger", () => {
  beforeEach(() => {
    readSessionFile.mockReset();
    readSessionFile.mockResolvedValue({
      path: "src/utils.rs",
      name: "utils.rs",
      status: "text",
      size: 24,
      content: "let a = 1;\nlet b = 2;\nlet c = 3;",
    });
  });

  it("opens the preview and attaches nothing when a candidate is chosen without a typed range", async () => {
    const { onAddFileReference, onChange } = renderComposer("@utils");

    fireEvent.click(screen.getByText("src/utils.rs"));
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    expect(onAddFileReference).not.toHaveBeenCalled();
    // The draft must be untouched while the decision is still pending.
    expect(onChange).not.toHaveBeenCalled();
  });

  it("attaches directly and opens no preview when the mention already carries a range", async () => {
    const { onAddFileReference, onChange } = renderComposer("@src/utils.rs:10-50");

    fireEvent.click(screen.getByText("src/utils.rs"));
    expect(onAddFileReference).toHaveBeenCalledWith({ name: "utils.rs", path: "src/utils.rs" }, { startLine: 10, endLine: 50 });
    expect(onChange).toHaveBeenCalledWith("@src/utils.rs:10-50 ");
    expect(readSessionFile).not.toHaveBeenCalled();
    expect(screen.queryByTestId("preview-line-1")).toBeNull();
  });

  it("attaches the picked range and rewrites the mention when the preview is confirmed", async () => {
    const { onAddFileReference, onChange } = renderComposer("@utils");

    fireEvent.click(screen.getByText("src/utils.rs"));
    await waitFor(() => expect(screen.getByTestId("preview-line-2")).toBeTruthy());
    fireEvent.click(screen.getByTestId("preview-line-2"));
    fireEvent.click(screen.getByText("引用所选范围"));

    expect(onAddFileReference).toHaveBeenCalledWith({ name: "utils.rs", path: "src/utils.rs" }, { startLine: 2, endLine: 2 });
    expect(onChange).toHaveBeenCalledWith("@src/utils.rs:2 ");
  });

  it("leaves the draft untouched when the preview is dismissed", async () => {
    const { onAddFileReference, onChange } = renderComposer("@utils");

    fireEvent.click(screen.getByText("src/utils.rs"));
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    fireEvent.click(screen.getByText("取消"));

    await waitFor(() => expect(screen.queryByTestId("preview-line-1")).toBeNull());
    expect(onAddFileReference).not.toHaveBeenCalled();
    expect(onChange).not.toHaveBeenCalled();
  });

  it("closes the preview when the session changes", async () => {
    const { onAddFileReference, switchSession } = renderComposer("@utils");
    fireEvent.click(screen.getByText("src/utils.rs"));
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());

    expect(readSessionFile).toHaveBeenCalledWith("session-1", "src/utils.rs");
    switchSession("session-2");

    // Asserting on the read, not on the dialog disappearing: the dialog is lazy-loaded, so
    // it blinks out during any re-render and a "no longer visible" check passes even when
    // the preview is still pending. What must not happen is the old path being re-read
    // against the new session — that is what silently shows a different file.
    await new Promise((resolve) => { setTimeout(resolve, 50); });
    expect(readSessionFile).not.toHaveBeenCalledWith("session-2", expect.anything());
    expect(readSessionFile).toHaveBeenCalledTimes(1);
    // And the dialog itself is gone: a preview belonging to another session must not stay
    // on screen offering to attach a file the user is no longer looking at. The wait above
    // is what makes this meaningful — the lazy dialog would have had time to come back.
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(onAddFileReference).not.toHaveBeenCalled();
  });

  it("confines focus to the dialog while it is open", async () => {
    renderComposer("@utils");
    fireEvent.click(screen.getByText("src/utils.rs"));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeTruthy());
    expect(screen.getByRole("dialog").contains(document.activeElement)).toBe(true);
  });
});
