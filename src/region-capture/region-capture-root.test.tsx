// @vitest-environment jsdom

import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";

const { cancel, commit } = vi.hoisted(() => ({
  cancel: vi.fn(async () => undefined),
  commit: vi.fn(async () => undefined),
}));

vi.mock("../services/runtime-local-media-client", () => ({
  localMediaService: {
    cancelScreenshotSelection: cancel,
    commitScreenshotSelection: commit,
  },
}));

import { RegionCaptureRoot } from "./region-capture-root";

describe("RegionCaptureRoot", () => {
  beforeEach(async () => {
    cancel.mockClear();
    commit.mockClear();
    window.history.replaceState({}, "", "/?surface=region-capture&run=run1&display=display1");
    await activateAppLanguage("zh-CN");
  });

  it("commits a bounded drag with opaque run and display tokens", async () => {
    renderWithAppProviders(<RegionCaptureRoot />);
    const surface = screen.getByTestId("region-capture-surface");

    fireEvent.pointerDown(surface, { button: 0, clientX: 20, clientY: 30, pointerId: 1 });
    fireEvent.pointerMove(surface, { clientX: 220, clientY: 130, pointerId: 1 });
    fireEvent.pointerUp(surface, { clientX: 220, clientY: 130, pointerId: 1 });

    await waitFor(() =>
      expect(commit).toHaveBeenCalledWith({
        runId: "run1",
        displayToken: "display1",
        x: 20,
        y: 30,
        width: 200,
        height: 100,
      }),
    );
  });

  it("cancels with Escape without committing pixels", async () => {
    renderWithAppProviders(<RegionCaptureRoot />);
    fireEvent.keyDown(window, { key: "Escape" });

    await waitFor(() => expect(cancel).toHaveBeenCalledWith({ runId: "run1" }));
    expect(commit).not.toHaveBeenCalled();
  });

  it("rejects a below-minimum drag", () => {
    renderWithAppProviders(<RegionCaptureRoot />);
    const surface = screen.getByTestId("region-capture-surface");
    fireEvent.pointerDown(surface, { button: 0, clientX: 20, clientY: 20, pointerId: 1 });
    fireEvent.pointerUp(surface, { clientX: 24, clientY: 24, pointerId: 1 });

    expect(commit).not.toHaveBeenCalled();
  });

  it("normalizes reverse drags and clamps them to the overlay", async () => {
    renderWithAppProviders(<RegionCaptureRoot />);
    const surface = screen.getByTestId("region-capture-surface");
    fireEvent.pointerDown(surface, { button: 0, clientX: 220, clientY: 130, pointerId: 2 });
    fireEvent.pointerUp(surface, { clientX: -20, clientY: -30, pointerId: 2 });

    await waitFor(() =>
      expect(commit).toHaveBeenCalledWith(expect.objectContaining({
        x: 0,
        y: 0,
        width: 220,
        height: 130,
      })),
    );
  });

  it("cancels on a secondary click", async () => {
    renderWithAppProviders(<RegionCaptureRoot />);
    fireEvent.pointerDown(screen.getByTestId("region-capture-surface"), { button: 2 });

    await waitFor(() => expect(cancel).toHaveBeenCalledWith({ runId: "run1" }));
  });
});
