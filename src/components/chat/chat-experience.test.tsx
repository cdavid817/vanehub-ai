import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { ChatInputBox } from "./ChatInputBox";
import { MessageItem } from "./MessageItem";

const config: ChatConfig = {
  agentId: "codex-cli",
  interactionMode: "cli",
  permissionMode: "default",
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
};

describe("chat Mermaid and file references", () => {
  it("routes Mermaid fenced code blocks through the diagram renderer", () => {
    const html = renderToString(
      <MessageItem
        message={{
          id: "message-1",
          sessionId: "session-1",
          role: "assistant",
          content: "```mermaid\ngraph TD\nA-->B\n```",
          status: "completed",
          createdAt: "2026-07-18T00:00:00.000Z",
          updatedAt: "2026-07-18T00:00:00.000Z",
        }}
      />,
    );

    expect(html).toContain("正在渲染 Mermaid 图表");
  });

  it("shows bounded @ file candidates and selected reference chips", () => {
    const html = renderToString(
      <ChatInputBox
        agents={[agent]}
        availableModes={["default"]}
        availableModels={[]}
        availableReasoning={["low", "medium", "high", "max"]}
        config={config}
        fileReferenceCandidates={[
          { name: "README.md", path: "README.md", kind: "markdown" },
          { name: "notes.txt", path: "docs/notes.txt", kind: "text" },
        ]}
        fileReferences={[{ id: "README.md", path: "README.md", name: "README.md" }]}
        isStreaming={false}
        onAddFileReference={vi.fn()}
        onChange={vi.fn()}
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
        value="@notes"
      />,
    );

    expect(html).toContain("README.md");
    expect(html).toContain("docs/notes.txt");
  });
});
