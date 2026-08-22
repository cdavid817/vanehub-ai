// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { ChatMessage } from "../types/chat";
import { TerminalTab } from "./terminal-tab";

function message(id: string, seatId: string | undefined, toolName: string): ChatMessage {
  return {
    id,
    sessionId: "session-1",
    role: "assistant",
    speakerSeatId: seatId,
    content: "",
    status: "completed",
    toolUse: [{ id: `${id}-tool`, name: toolName, status: "completed" }],
    createdAt: "2026-08-22T10:00:00.000Z",
    updatedAt: "2026-08-22T10:00:00.000Z",
    sessionSequence: 1,
    executionRunId: null,
  };
}

describe("TerminalTab seat scope", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  const messages = [
    message("m1", "seat-planner", "planner_tool"),
    message("m2", "seat-builder", "builder_tool"),
    message("m3", undefined, "unattributed_tool"),
  ];

  it("shows every seat's activity when no seat is selected", () => {
    renderWithAppProviders(<TerminalTab messages={messages} partial={false} />);

    expect(screen.getByText("planner_tool")).toBeDefined();
    expect(screen.getByText("builder_tool")).toBeDefined();
    expect(screen.getByText("unattributed_tool")).toBeDefined();
  });

  it("shows only the selected seat's activity", () => {
    renderWithAppProviders(<TerminalTab messages={messages} partial={false} seatId="seat-builder" />);

    expect(screen.getByText("builder_tool")).toBeDefined();
    expect(screen.queryByText("planner_tool")).toBeNull();
  });

  // Attributing an unlabelled message to whichever seat is selected would invent evidence.
  it("does not attribute unattributed activity to the selected seat", () => {
    renderWithAppProviders(<TerminalTab messages={messages} partial={false} seatId="seat-builder" />);

    expect(screen.queryByText("unattributed_tool")).toBeNull();
  });
});
