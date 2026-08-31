// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  clampInspectorWidth,
  clampNavigationWidth,
  clampRuntimeHeight,
  patchDestinationLayoutPreference,
  readInitialSessionsLayout,
} from "./workbench-layout-preferences";

const DEFAULTS = {
  navigationWidth: 280,
  inspectorWidth: 300,
  inspectorOpen: false,
  runtimeHeight: 260,
  preferredRuntimeTab: undefined,
};

describe("workbench layout preferences", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("defaults to the unresized bounds, a closed inspector, and no preferred runtime tab when nothing is stored", () => {
    expect(readInitialSessionsLayout()).toEqual(DEFAULTS);
  });

  // Distinct from the min-clamp test below: 232/420 used to coincide with the old default,
  // so a default/min mixup could never have failed a test. 280 sits strictly inside the
  // 256-400 range, so this only passes if the real default is read, not the floor.
  it("the default width is not the same as the minimum bound", () => {
    expect(readInitialSessionsLayout().navigationWidth).not.toBe(clampNavigationWidth(0));
  });

  it("migrates the legacy sidebar width key into the V2 shape's initial read", () => {
    localStorage.setItem("vanehub.session-sidebar.width.v1", "340");
    expect(readInitialSessionsLayout().navigationWidth).toBe(340);
  });

  it("prefers a V2 navigation width over the legacy key once one has been written", () => {
    localStorage.setItem("vanehub.session-sidebar.width.v1", "340");
    patchDestinationLayoutPreference("sessions", { navigationWidth: 260 });
    expect(readInitialSessionsLayout().navigationWidth).toBe(260);
  });

  it("clamps a malformed legacy width instead of blocking startup", () => {
    localStorage.setItem("vanehub.session-sidebar.width.v1", "not-a-number");
    expect(readInitialSessionsLayout().navigationWidth).toBe(280);
  });

  it("clamps stored widths to their bounds", () => {
    patchDestinationLayoutPreference("sessions", { navigationWidth: 5000, inspectorWidth: -10 });
    expect(readInitialSessionsLayout()).toEqual({ ...DEFAULTS, navigationWidth: 400, inspectorWidth: 260 });
  });

  it("persists and restores the inspector's open preference independently of width", () => {
    patchDestinationLayoutPreference("sessions", { preferredInspectorOpen: true });
    expect(readInitialSessionsLayout().inspectorOpen).toBe(true);
    patchDestinationLayoutPreference("sessions", { preferredInspectorOpen: false });
    expect(readInitialSessionsLayout().inspectorOpen).toBe(false);
  });

  it("patches without clobbering another field already stored for the same destination", () => {
    patchDestinationLayoutPreference("sessions", { navigationWidth: 300 });
    patchDestinationLayoutPreference("sessions", { inspectorWidth: 350 });
    expect(readInitialSessionsLayout()).toEqual({ ...DEFAULTS, navigationWidth: 300, inspectorWidth: 350 });
  });

  it("falls back to defaults on a corrupted V2 record rather than throwing", () => {
    localStorage.setItem("vanehub.workbench.layout.v2", "{not json");
    expect(readInitialSessionsLayout()).toEqual(DEFAULTS);
  });

  it("falls back to defaults on a wrong-version V2 record", () => {
    localStorage.setItem("vanehub.workbench.layout.v2", JSON.stringify({ version: 1, destination: {} }));
    expect(readInitialSessionsLayout()).toEqual(DEFAULTS);
  });

  it("clamps standalone width helpers to the same bounds used at read time", () => {
    expect(clampNavigationWidth(0)).toBe(256);
    expect(clampNavigationWidth(5000)).toBe(400);
    expect(clampInspectorWidth(0)).toBe(260);
    expect(clampInspectorWidth(5000)).toBe(480);
  });

  it("persists and restores the Runtime Panel's height, clamped to its bounds", () => {
    patchDestinationLayoutPreference("sessions", { runtimeHeight: 5000 });
    expect(readInitialSessionsLayout().runtimeHeight).toBe(640);
    patchDestinationLayoutPreference("sessions", { runtimeHeight: 320 });
    expect(readInitialSessionsLayout().runtimeHeight).toBe(320);
  });

  it("persists and restores the preferred Runtime Panel tab without touching height", () => {
    patchDestinationLayoutPreference("sessions", { runtimeHeight: 320, preferredRuntimeTab: "shell" });
    const layout = readInitialSessionsLayout();
    expect(layout.preferredRuntimeTab).toBe("shell");
    expect(layout.runtimeHeight).toBe(320);
  });

  it("clamps the standalone runtime height helper to the same bounds used at read time", () => {
    expect(clampRuntimeHeight(0)).toBe(160);
    expect(clampRuntimeHeight(5000)).toBe(640);
  });
});
