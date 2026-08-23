import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock, openMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));

import { tauriLocalMediaClient } from "./tauri-local-media-client";

const STAGED = {
  stagedInputId: "staged-1",
  displayName: "fixture.png",
  mediaType: "image" as const,
  byteLength: 10,
};

/**
 * Which failures may open a real dialog.
 *
 * Only one may: the code the fixture command answers when the fixture runtime was never activated,
 * which is the ordinary Desktop Smoke layer running the same build. Every other failure is a
 * defect, and opening a dialog no headless runner can answer would turn it into a hang instead of
 * a report.
 */
describe("the desktop OCR picker seam", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openMock.mockReset();
    vi.stubEnv("VITE_DESKTOP_E2E", "1");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("stages the fixture path without opening a dialog", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "fixture_local_media_ocr_source") return "/fixtures/invoice.png";
      return STAGED;
    });

    const staged = await tauriLocalMediaClient.selectAndStageOcrSource();

    expect(openMock).not.toHaveBeenCalled();
    expect(staged).toEqual(STAGED);
    expect(invokeMock).toHaveBeenCalledWith("stage_local_media_ocr_source", {
      request: { path: "/fixtures/invoice.png" },
    });
  });

  it("falls back to the real dialog only for the unavailable code", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "fixture_local_media_ocr_source") {
        throw new Error("FIXTURE_OCR_SOURCE_UNAVAILABLE");
      }
      return STAGED;
    });
    openMock.mockResolvedValue("C:/chosen/by/a/human.png");

    const staged = await tauriLocalMediaClient.selectAndStageOcrSource();

    expect(openMock).toHaveBeenCalledTimes(1);
    expect(staged).toEqual(STAGED);
  });

  it.each([
    ["a different stable code", "MODEL_NOT_FOUND"],
    ["a registry miss", "Command fixture_local_media_ocr_source not found"],
  ])("rethrows %s instead of opening a dialog", async (_label, message) => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "fixture_local_media_ocr_source") throw new Error(message);
      return STAGED;
    });

    await expect(tauriLocalMediaClient.selectAndStageOcrSource()).rejects.toThrow();
    expect(openMock).not.toHaveBeenCalled();
  });

  it("rethrows a transport failure instead of opening a dialog", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "fixture_local_media_ocr_source") {
        // No stable code at all: an IPC fault, not a fixture answer.
        throw new TypeError("channel closed");
      }
      return STAGED;
    });

    await expect(tauriLocalMediaClient.selectAndStageOcrSource()).rejects.toThrow("channel closed");
    expect(openMock).not.toHaveBeenCalled();
  });

  it("never asks the fixture command outside a desktop test build", async () => {
    vi.stubEnv("VITE_DESKTOP_E2E", undefined);
    openMock.mockResolvedValue("C:/chosen/by/a/human.png");
    invokeMock.mockResolvedValue(STAGED);

    await tauriLocalMediaClient.selectAndStageOcrSource();

    expect(openMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).not.toHaveBeenCalledWith("fixture_local_media_ocr_source");
  });
});
