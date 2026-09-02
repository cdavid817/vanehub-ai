// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import { defaultWorkBoardQuery, type WorkBoardQuery } from "./work-board-query";
import {
  applyWorkBoardSavedView, captureWorkBoardSavedView, readWorkBoardSavedViews,
  WORK_BOARD_SAVED_VIEW_NAME_MAX_LENGTH, writeWorkBoardSavedViews, type WorkBoardSavedView,
} from "./work-board-saved-views";

const STORAGE_KEY = "vanehub.work-board.saved-views.v1";

const urgentQuery: WorkBoardQuery = {
  ...defaultWorkBoardQuery,
  text: "should never be persisted",
  priority: "urgent",
  source: "session",
  due: "overdue",
  stage: "in_progress",
  project: "D:/app",
  sort: "priority",
  grouping: "none",
  presentation: "list",
};

beforeEach(() => {
  localStorage.clear();
});

describe("captureWorkBoardSavedView", () => {
  it("captures every bounded query dimension but never the free-text search", () => {
    const view = captureWorkBoardSavedView(urgentQuery, "My urgent items", "view-1");
    expect(view).toEqual({
      id: "view-1", name: "My urgent items", project: "D:/app", source: "session", priority: "urgent",
      due: "overdue", stage: "in_progress", sort: "priority", grouping: "none", presentation: "list",
    });
    expect(view).not.toHaveProperty("text");
  });

  it("trims and length-caps the name so it cannot become a dumping ground for unrestricted content", () => {
    const long = "x".repeat(WORK_BOARD_SAVED_VIEW_NAME_MAX_LENGTH + 50);
    const view = captureWorkBoardSavedView(defaultWorkBoardQuery, `  ${long}  `, "view-2");
    expect(view.name).toHaveLength(WORK_BOARD_SAVED_VIEW_NAME_MAX_LENGTH);
  });
});

describe("applyWorkBoardSavedView", () => {
  it("restores every stored dimension and resets free-text search to empty", () => {
    const view = captureWorkBoardSavedView(urgentQuery, "My urgent items", "view-1");
    const restored = applyWorkBoardSavedView(view);
    expect(restored).toEqual({ ...urgentQuery, text: "" });
  });
});

describe("read/writeWorkBoardSavedViews round-trip", () => {
  it("returns an empty list before anything has been saved", () => {
    expect(readWorkBoardSavedViews()).toEqual([]);
  });

  it("round-trips a saved view through write then read", () => {
    const view = captureWorkBoardSavedView(urgentQuery, "My urgent items", "view-1");
    writeWorkBoardSavedViews([view]);
    expect(readWorkBoardSavedViews()).toEqual([view]);
  });

  it("survives a whole-payload version bump gracefully by discarding the unreadable format", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 2, views: [captureWorkBoardSavedView(urgentQuery, "Future format", "view-1")] }));
    expect(readWorkBoardSavedViews()).toEqual([]);
  });

  it("drops one malformed entry within an otherwise-current payload without losing the others", () => {
    const good = captureWorkBoardSavedView(urgentQuery, "Good view", "view-good");
    const corrupt = { ...good, id: "view-bad", priority: "not-a-real-priority" };
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, views: [good, corrupt] }));
    expect(readWorkBoardSavedViews()).toEqual([good]);
  });

  it("fails closed to an empty list rather than throwing on unparseable JSON", () => {
    localStorage.setItem(STORAGE_KEY, "not json");
    expect(readWorkBoardSavedViews()).toEqual([]);
  });

  it("deletes are just a write of the filtered list -- confirmed by round-tripping one out", () => {
    const keep = captureWorkBoardSavedView(urgentQuery, "Keep me", "view-keep");
    const drop: WorkBoardSavedView = { ...keep, id: "view-drop", name: "Drop me" };
    writeWorkBoardSavedViews([keep, drop]);
    writeWorkBoardSavedViews(readWorkBoardSavedViews().filter((view) => view.id !== "view-drop"));
    expect(readWorkBoardSavedViews()).toEqual([keep]);
  });
});
