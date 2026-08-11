// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import "../../i18n";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { MessageItem } from "./MessageItem";
import type { ChatMessage } from "../../types/chat";
import type { MessageSpeaker } from "../../services/message-speaker";

function message(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: "m1",
    sessionId: "s1",
    role: "assistant",
    content: "已完成。",
    status: "completed",
    createdAt: "2026-08-06T10:00:00Z",
    updatedAt: "2026-08-06T10:00:00Z",
    ...overrides,
  };
}

const reviewer: MessageSpeaker = {
  agentId: "codex-cli",
  avatar: "🔍",
  color: "#C77D3A",
  roleName: "代码审查",
  agentName: "Codex CLI",
  crossFamilyReviewer: true,
};

beforeEach(async () => {
  await activateAppLanguage("zh-CN");
});

describe("MessageItem speaker identity", () => {
  it("names both the role and the Agent for a seat that has a role", () => {
    renderWithAppProviders(<MessageItem message={message()} speaker={reviewer} />);
    expect(screen.getByText("代码审查")).toBeTruthy();
    expect(screen.getByText("· Codex CLI")).toBeTruthy();
    expect(screen.getByTestId("message-role-color").getAttribute("fill")).toBe("#C77D3A");
  });

  it("falls back to the semantic text colour for an invalid captured role colour", () => {
    renderWithAppProviders(<MessageItem message={message()} speaker={{ ...reviewer, color: "not-a-colour" }} />);
    expect(screen.getByTestId("message-role-color").getAttribute("fill")).toBe("currentColor");
  });

  it("marks a cross-family reviewer", () => {
    renderWithAppProviders(<MessageItem message={message()} speaker={reviewer} />);
    expect(screen.getByText("跨家族")).toBeTruthy();
  });

  it("falls back to the Agent name when the seat has no role", () => {
    const plain: MessageSpeaker = { ...reviewer, roleName: null, crossFamilyReviewer: false };
    renderWithAppProviders(<MessageItem message={message()} speaker={plain} />);
    expect(screen.getByText("Codex CLI")).toBeTruthy();
    expect(screen.queryByText("跨家族")).toBeNull();
  });

  // The whole point of making `speaker` optional: an existing single-Agent session must look
  // exactly as it did before seats existed.
  it("renders the original generic label when there is no speaker", () => {
    renderWithAppProviders(<MessageItem message={message()} />);
    expect(screen.getByText("Agent")).toBeTruthy();
    expect(screen.queryByText("代码审查")).toBeNull();
  });

  it("never shows a speaker on a user message", () => {
    renderWithAppProviders(<MessageItem message={message({ role: "user" })} speaker={reviewer} />);
    expect(screen.getByText("你")).toBeTruthy();
    expect(screen.queryByText("代码审查")).toBeNull();
  });
});
