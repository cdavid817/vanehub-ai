import { describe, expect, it } from "vitest";
import { i18n } from "../i18n";
import { matchWorkspaceSurfaces, workspaceSurfaceEntries } from "./workspace-search-entries";

describe("workspace search entries", () => {
  it("lists every demoted surface with a localized label in each registered locale", () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"]) {
      const entries = workspaceSurfaceEntries(i18n.getFixedT(locale));
      expect(entries.map((entry) => entry.id)).toEqual(["inbox", "board", "activity", "loops", "scheduled", "goals", "evaluation"]);
      for (const entry of entries) expect(entry.label, `${locale}:${entry.id}`).not.toMatch(/\./);
    }
  });

  it("matches by localized label and by locale-independent keywords", () => {
    const entries = workspaceSurfaceEntries(i18n.getFixedT("zh-CN"));
    expect(matchWorkspaceSurfaces("看板", entries).map((entry) => entry.id)).toEqual(["board"]);
    expect(matchWorkspaceSurfaces("board", entries).map((entry) => entry.id)).toEqual(["board"]);
    expect(matchWorkspaceSurfaces("评测", entries).map((entry) => entry.id)).toEqual(["evaluation"]);
    expect(matchWorkspaceSurfaces("loop", entries).map((entry) => entry.id)).toEqual(["loops"]);
    expect(matchWorkspaceSurfaces("定时", entries).map((entry) => entry.id)).toEqual(["scheduled"]);
    expect(matchWorkspaceSurfaces("goal", entries).map((entry) => entry.id)).toEqual(["goals"]);
    expect(matchWorkspaceSurfaces("   ", entries)).toEqual([]);
    expect(matchWorkspaceSurfaces("zzz", entries)).toEqual([]);
  });

  it("points each entry at its new host", () => {
    const byId = Object.fromEntries(workspaceSurfaceEntries(i18n.getFixedT("en")).map((entry) => [entry.id, entry.target]));
    expect(byId.board).toEqual({ kind: "workspace", location: { destination: "inbox", view: "board" } });
    expect(byId.scheduled).toEqual({ kind: "workspace", location: { destination: "automations", view: "scheduled" } });
    expect(byId.evaluation).toEqual({ kind: "settings", pageId: "evaluation" });
  });
});
