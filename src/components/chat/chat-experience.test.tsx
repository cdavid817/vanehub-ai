import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { AgentRegistryEntry } from "../../types/agent";
import type { ChatConfig } from "../../types/chat";
import { ChatInputBox } from "./ChatInputBox";
import { MessageItem } from "./MessageItem";
import { anchoredScrollTop } from "./use-conversation-window-model";

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

describe("chat Mermaid and file references", () => {
  it("renders assistant Markdown with normal whitespace and bounded overflow", () => {
    const html = renderToString(
      <MessageItem
        message={{
          id: "message-streamed",
          sessionId: "session-1",
          role: "assistant",
          content: "Deep\nSeek streamed reply",
          status: "completed",
          createdAt: "2026-08-03T00:00:00.000Z",
          updatedAt: "2026-08-03T00:00:00.000Z",
          sessionSequence: 1,
          executionRunId: null,
        }}
      />,
    );

    expect(html).toContain("whitespace-normal");
    expect(html).toContain("wrap-break-word");
    expect(html).not.toContain("whitespace-pre-wrap");
  });

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
          sessionSequence: 1,
          executionRunId: null,
        }}
      />,
    );

    expect(html).toContain("正在渲染 Mermaid 图表");
  });

  it("shows ranked @ file candidates including source files, plus selected reference chips", () => {
    const html = renderToString(
      <ChatInputBox
        agents={[agent]}
        availableModes={["inherit"]}
        availableModels={[]}
        availableReasoning={["low", "medium", "high", "max"]}
        config={config}
        fileReferenceCandidates={[
          { name: "README.md", path: "README.md" },
          { name: "session_search.rs", path: "src-tauri/src/session_search.rs" },
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
        value="@session"
      />,
    );

    expect(html).toContain("README.md");
    // Candidates arrive already ranked from the native search; a source file must be offerable.
    expect(html).toContain("src-tauri/src/session_search.rs");
    expect(html).toContain('data-testid="wechat-style-composer"');
    expect(html).not.toContain("rounded-xl border border-border bg-background shadow-xs");
    expect(html).toContain('data-testid="composer-toolbar"');
    expect(html).toContain('data-testid="effective-execution-policy"');
  });

  it("distinguishes participant mentions from file completions", () => {
    const html = renderToString(
      <ChatInputBox
        agents={[agent]}
        availableModes={["inherit"]}
        availableModels={[]}
        availableReasoning={["medium"]}
        config={config}
        fileReferenceCandidates={[]}
        fileReferences={[]}
        isStreaming={false}
        participantMentions={[{ mention: "Reviewer", roleName: "Reviewer", agentName: "Codex", modelFamily: "openai", avatar: "🔍" }]}
        onAddFileReference={vi.fn()} onChange={vi.fn()} onClear={vi.fn()}
        onConfigAgentChange={vi.fn()} onConfigLongContextChange={vi.fn()}
        onConfigModeChange={vi.fn()} onConfigModelChange={vi.fn()}
        onConfigProviderChange={vi.fn()} onConfigReasoningChange={vi.fn()}
        onConfigStreamingChange={vi.fn()} onConfigThinkingChange={vi.fn()}
        onRemoveFileReference={vi.fn()} onStop={vi.fn()} onSubmit={vi.fn()}
        value="@rev"
      />,
    );
    expect(html).toContain("Reviewer");
    expect(html).toContain('role="group"');
    expect(html).not.toContain("docs/");
  });

  it("pins the latest message and preserves history offset after message reflow", () => {
    expect(anchoredScrollTop(true, 1_000, 1_240, 500)).toBe(1_240);
    expect(anchoredScrollTop(false, 1_000, 1_240, 500)).toBe(740);
    expect(anchoredScrollTop(false, 1_000, 800, 100)).toBe(0);
  });
});
