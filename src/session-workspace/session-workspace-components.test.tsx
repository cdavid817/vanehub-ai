import { renderToStaticMarkup } from "react-dom/server";
import ReactMarkdown from "react-markdown";
import { beforeAll, describe, expect, it } from "vitest";
import { i18n } from "../i18n";
import type { ChatMessage } from "../types/chat";
import { ReportTab } from "./report-tab";
import { SessionTabBar } from "./session-tab-bar";
import { TerminalTab, toolUseCount } from "./terminal-tab";

const message: ChatMessage = {
  id: "message-1",
  sessionId: "session-1",
  role: "assistant",
  content: "done",
  status: "completed",
  toolUse: [{ id: "tool-1", name: "read_file", input: { path: "README.md" }, status: "completed" }],
  tokenUsage: { input: 12, output: 8 },
  createdAt: "2026-07-17T00:00:00.000Z",
  updatedAt: "2026-07-17T00:00:01.000Z",
};

describe("session workspace components", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("en");
  });
  it("renders all tab labels and the terminal badge", () => {
    const html = renderToStaticMarkup(<SessionTabBar activeTab="chat" badges={{ terminal: 1 }} onActivate={() => undefined} />);
    expect(html).toContain("Chat");
    expect(html).toContain("Changes");
    expect(html).toContain("Report");
    expect(html).toContain("1");
  });

  it("renders tool execution cards and report values", () => {
    expect(toolUseCount([message])).toBe(1);
    expect(renderToStaticMarkup(<TerminalTab messages={[message]} partial={false} />)).toContain("read_file");
    const report = renderToStaticMarkup(<ReportTab messages={[message]} partial={false} />);
    expect(report).toContain("12");
    expect(report).toContain("read_file");
    expect(report).toContain("Message status");
    expect(report).toContain("Completed");
    expect(report).toContain("Completion");
  });

  it("does not render raw Markdown HTML", () => {
    const html = renderToStaticMarkup(<ReactMarkdown skipHtml>{"# Safe\n<script>alert('x')</script>"}</ReactMarkdown>);
    expect(html).toContain("Safe");
    expect(html).not.toContain("<script");
  });
});
