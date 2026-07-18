import { renderToString } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";
import { ConversationCard } from "./main-layout";

describe("ConversationCard IM source", () => {
  it("renders only the localized platform label", () => {
    const session: Session = {
      id: "session-im",
      title: "External task",
      agentId: "codex-cli",
      interactionMode: "cli",
      lifecycleState: "idle",
      folder: null,
      projectPath: "D:\\code\\project",
      worktreePath: null,
      worktreeName: null,
      worktreeBranch: null,
      runtimeSessionId: null,
      categoryId: null,
      source: { kind: "im", connector: "dingtalk" },
      pinned: false,
      archived: false,
      createdAt: "2026-07-17T00:00:00.000Z",
      updatedAt: "2026-07-17T00:00:00.000Z",
    };
    const html = renderToString(
      <ConversationCard active={false} language="en" lifecycleLabel="Idle" onContextMenu={vi.fn()} onSelect={vi.fn()} session={session} sourceLabel="DingTalk" />,
    );
    expect(html).toContain("DingTalk");
    expect(html).not.toContain("external-chat");
  });
});
