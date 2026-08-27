import { beforeEach, describe, expect, it, vi } from "vitest";

import { localMediaErrorCodes } from "../types/local-media";
import type { LocalMediaService } from "./local-media-service";

const { invokeMock, openMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openMock }));

import { tauriLocalMediaClient } from "./tauri-local-media-client";
import { webLocalMediaClient } from "./web-local-media-client";

/** Every method both adapters must answer. Kept as data so a new capability cannot skip one. */
const METHODS: Array<keyof LocalMediaService> = [
  "isAvailable",
  "getProfile",
  "saveProfile",
  "validateProfile",
  "getStatus",
  "listAudioDevices",
  "discoverPythonEnvironments",
  "probeEngine",
  "selectProfilePath",
  "selectAndStageOcrSource",
  "selectAndStageScreenshotRegion",
  "commitScreenshotSelection",
  "cancelScreenshotSelection",
  "cancelActiveScreenshotSelection",
  "discardStagedOcrSource",
  "startOcr",
  "startRecording",
  "stopRecordingAndTranscribe",
  "cancelRecording",
  "startTts",
  "stopPlayback",
  "cancelOperation",
  "getOperationResult",
];

describe("local media adapters", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openMock.mockReset();
    invokeMock.mockResolvedValue(null);
  });

  it("implements the same surface in both runtimes", () => {
    // The two clients are chosen at runtime by the same factory, so a method present on one and
    // absent on the other is a crash in whichever build the developer is not using.
    for (const method of METHODS) {
      expect(typeof tauriLocalMediaClient[method], `tauri:${method}`).toBe("function");
      expect(typeof webLocalMediaClient[method], `web:${method}`).toBe("function");
    }
    expect(Object.keys(tauriLocalMediaClient).sort()).toEqual(
      Object.keys(webLocalMediaClient).sort(),
    );
  });

  describe("the Tauri adapter", () => {
    it("wraps every payload in the `request` envelope the commands expect", async () => {
      await tauriLocalMediaClient.startOcr({ stagedInputId: "s1", composerScopeId: "c1" });

      expect(invokeMock).toHaveBeenCalledWith("start_local_media_ocr", {
        request: { stagedInputId: "s1", composerScopeId: "c1" },
      });
    });

    it("sends the optimistic revision with a save", async () => {
      const profile = { profileId: "default", revision: 7 } as never;
      await tauriLocalMediaClient.saveProfile({ profile, expectedRevision: 7 });

      expect(invokeMock).toHaveBeenCalledWith("save_local_media_profile", {
        request: { profile, expectedRevision: 7 },
      });
    });

    it("normalizes an absent playback id to null rather than omitting the field", async () => {
      await tauriLocalMediaClient.stopPlayback({});

      // `undefined` disappears from a JSON payload; the command's DTO has a required nullable
      // field, so an omitted key deserializes as a missing field rather than as "stop everything".
      expect(invokeMock).toHaveBeenCalledWith("stop_local_media_playback", {
        request: { playbackId: null },
      });
    });

    it("stages the picked file and never returns its path", async () => {
      openMock.mockResolvedValue("C:/Users/someone/scan.png");
      invokeMock.mockResolvedValue({
        stagedInputId: "staged-1",
        displayName: "scan.png",
        mediaType: "image",
        byteLength: 10,
      });

      const staged = await tauriLocalMediaClient.selectAndStageOcrSource();

      expect(invokeMock).toHaveBeenCalledWith("stage_local_media_ocr_source", {
        request: { path: "C:/Users/someone/scan.png" },
      });
      expect(JSON.stringify(staged)).not.toContain("someone");
    });

    it("returns null when the picker is dismissed and stages nothing", async () => {
      openMock.mockResolvedValue(null);

      expect(await tauriLocalMediaClient.selectAndStageOcrSource()).toBeNull();
      expect(invokeMock).not.toHaveBeenCalled();
    });

    it("offers only the formats admission actually accepts", async () => {
      openMock.mockResolvedValue(null);
      await tauriLocalMediaClient.selectAndStageOcrSource();

      // Offering TIFF here would only produce a rejection after the user chose a file.
      const [options] = openMock.mock.calls[0];
      expect(options.filters[0].extensions).toEqual(["png", "jpg", "jpeg", "bmp", "pdf"]);
    });

    it("asks for a directory or a file depending on the profile field", async () => {
      openMock.mockResolvedValue("/models/rec");
      await tauriLocalMediaClient.selectProfilePath({ kind: "directory" });
      expect(openMock.mock.calls[0][0]).toMatchObject({ directory: true, multiple: false });

      await tauriLocalMediaClient.selectProfilePath({ kind: "file" });
      expect(openMock.mock.calls[1][0]).toMatchObject({ directory: false, multiple: false });
    });

    it("passes a discriminated operation result through untouched", async () => {
      const result = { kind: "stt", result: { text: "hello" } };
      invokeMock.mockResolvedValue(result);

      expect(await tauriLocalMediaClient.getOperationResult("op-1")).toEqual(result);
    });

    it("reports a still-running operation as null rather than as an empty result", async () => {
      invokeMock.mockResolvedValue(null);

      // An empty result object would be indistinguishable from a finished operation that produced
      // nothing, and the composer would stop polling.
      expect(await tauriLocalMediaClient.getOperationResult("op-1")).toBeNull();
    });

    it("cancels through the operation id alone", async () => {
      await tauriLocalMediaClient.cancelOperation("op-9");

      expect(invokeMock).toHaveBeenCalledWith("cancel_local_media_operation", {
        request: { operationId: "op-9" },
      });
    });

    it("normalizes Python discovery and rejects malformed native payloads", async () => {
      const discovery = {
        availability: "available",
        reasonCode: null,
        candidates: [{
          executablePath: "/usr/bin/python3",
          version: { major: 3, minor: 12, patch: 2 },
          compatibility: "compatible",
          reasonCode: null,
          source: "path",
        }],
      };
      invokeMock.mockResolvedValueOnce(discovery);
      await expect(tauriLocalMediaClient.discoverPythonEnvironments()).resolves.toEqual(discovery);
      expect(invokeMock).toHaveBeenLastCalledWith("discover_local_media_python_environments");

      invokeMock.mockResolvedValueOnce({ ...discovery, candidates: [{ version: "3.12" }] });
      await expect(tauriLocalMediaClient.discoverPythonEnvironments()).rejects.toThrow(
        "LOCAL_MEDIA_DISCOVERY_INVALID_RESPONSE",
      );
    });
  });

  describe("the Web adapter", () => {
    it("reports itself unavailable", async () => {
      expect(await webLocalMediaClient.isAvailable()).toBe(false);
    });

    it("refuses every capability with the native-only code rather than simulating one", async () => {
      const refusing: Array<[string, () => Promise<unknown>]> = [
        ["probeEngine", () => webLocalMediaClient.probeEngine("ocr")],
        ["selectAndStageOcrSource", () => webLocalMediaClient.selectAndStageOcrSource()],
        ["startOcr", () => webLocalMediaClient.startOcr({ stagedInputId: "s", composerScopeId: "c" })],
        ["startRecording", () => webLocalMediaClient.startRecording({ composerScopeId: "c" })],
        ["startTts", () => webLocalMediaClient.startTts({ text: "x", composerScopeId: "c" })],
        ["stopPlayback", () => webLocalMediaClient.stopPlayback({})],
        ["cancelOperation", () => webLocalMediaClient.cancelOperation("op")],
        ["saveProfile", () => webLocalMediaClient.saveProfile({ profile: {} as never, expectedRevision: 0 })],
      ];

      for (const [name, call] of refusing) {
        // A mock transcript would be indistinguishable from a real one to anyone evaluating
        // whether the feature works.
        await expect(call(), name).rejects.toThrow("LOCAL_MEDIA_NATIVE_ONLY");
      }
    });

    it("uses a code the frontend can localize", () => {
      expect(localMediaErrorCodes).toContain("LOCAL_MEDIA_NATIVE_ONLY");
    });

    it("still renders the settings page by returning disabled defaults", async () => {
      const profile = await webLocalMediaClient.getProfile();

      expect(profile.enabled).toBe(false);
      expect(profile.ocr.enabled).toBe(false);
      expect(profile.stt.enabled).toBe(false);
      expect(profile.tts.enabled).toBe(false);
      expect(profile.revision).toBe(0);
    });

    it("reports every engine unavailable with the same code", async () => {
      const status = await webLocalMediaClient.getStatus();

      expect(status.nativeAvailable).toBe(false);
      expect(status.platformSupport).toBe("unsupported");
      expect(status.engines.map((engine) => engine.readiness)).toEqual([
        { state: "unavailable", code: "LOCAL_MEDIA_NATIVE_ONLY" },
        { state: "unavailable", code: "LOCAL_MEDIA_NATIVE_ONLY" },
        { state: "unavailable", code: "LOCAL_MEDIA_NATIVE_ONLY" },
      ]);
    });

    it("cannot produce a host path from a browser", async () => {
      expect(await webLocalMediaClient.selectProfilePath({ kind: "file" })).toBeNull();
    });

    it("reports Python discovery as native-only without inventing candidates", async () => {
      await expect(webLocalMediaClient.discoverPythonEnvironments()).resolves.toEqual({
        availability: "unavailable",
        reasonCode: "native_unavailable",
        candidates: [],
      });
    });

    it("never reaches a Tauri command", async () => {
      await webLocalMediaClient.getProfile();
      await webLocalMediaClient.getStatus();
      await webLocalMediaClient.listAudioDevices();
      await webLocalMediaClient.getOperationResult("op");

      expect(invokeMock).not.toHaveBeenCalled();
      expect(openMock).not.toHaveBeenCalled();
    });
  });
});
