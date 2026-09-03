// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import {
  readProjectsScrollTop, readProjectsView, writeProjectsScrollTop, writeProjectsView,
} from "./projects-view-state";

afterEach(() => sessionStorage.clear());

describe("projects view-state persistence", () => {
  it("returns null when nothing has been written yet", () => {
    expect(readProjectsView()).toBeNull();
  });

  it("round-trips each real WorkspaceView", () => {
    for (const view of ["recent", "all", "unavailable"] as const) {
      writeProjectsView(view);
      expect(readProjectsView()).toBe(view);
    }
  });

  it("discards a stored value that is not a real WorkspaceView", () => {
    sessionStorage.setItem("vanehub.projects.view.v1", "favorite");
    expect(readProjectsView()).toBeNull();
  });

  it("defaults scroll position to 0 when nothing has been written yet", () => {
    expect(readProjectsScrollTop()).toBe(0);
  });

  it("round-trips a written scroll position", () => {
    writeProjectsScrollTop(180);
    expect(readProjectsScrollTop()).toBe(180);
  });

  it("discards a negative or non-numeric stored scroll position", () => {
    sessionStorage.setItem("vanehub.projects.scroll.v1", "-5");
    expect(readProjectsScrollTop()).toBe(0);

    sessionStorage.setItem("vanehub.projects.scroll.v1", "not-a-number");
    expect(readProjectsScrollTop()).toBe(0);
  });
});
