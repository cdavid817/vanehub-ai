// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { activateAppLanguage } from "../../i18n";
import type { LocalMediaService } from "../../services/local-media-service";
import type {
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  ProfileFieldIssue,
} from "../../types/local-media";
import { renderWithAppProviders } from "../../test/render";
import { LocalMediaPage } from "./local-media-page";

const { serviceRef } = vi.hoisted(() => ({
  serviceRef: { current: null as LocalMediaService | null },
}));

vi.mock("../../services/runtime-local-media-client", () => ({
  get localMediaService() {
    if (!serviceRef.current) throw new Error("no service installed");
    return serviceRef.current;
  },
}));

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

function profile(overrides: Partial<LocalMediaProfile> = {}): LocalMediaProfile {
  return {
    profileId: "default",
    revision: 4,
    enabled: true,
    ocr: {
      enabled: true,
      pythonExecutable: "/opt/ocr/bin/python",
      paddleXConfigPath: null,
      textDetectionModelDir: "/models/det",
      textRecognitionModelDir: "/models/rec",
      textLineOrientationModelDir: null,
      language: "ch",
      device: "auto",
      maxPdfPages: 20,
    },
    stt: {
      enabled: false,
      pythonExecutable: "",
      modelDirectory: "",
      device: "auto",
      computeType: "auto",
      language: "auto",
      vadFilter: true,
      beamSize: 5,
      microphoneDeviceId: null,
      maxRecordingSeconds: 120,
    },
    tts: {
      enabled: false,
      pythonExecutable: "",
      modelKind: "vits",
      modelPath: "",
      tokensPath: "",
      lexiconPath: null,
      dataDir: null,
      dictDir: null,
      voicesPath: null,
      vocoderPath: null,
      ruleFsts: [],
      speakerId: 0,
      speed: 1,
      numThreads: 1,
      device: "cpu",
      outputDeviceId: null,
    },
    updatedAt: "2026-08-22T00:00:00Z",
    ...overrides,
  };
}

function status(overrides: Partial<LocalMediaRuntimeStatus> = {}): LocalMediaRuntimeStatus {
  return {
    nativeAvailable: true,
    platformSupport: "supported",
    enabled: true,
    profileRevision: 4,
    engines: [
      {
        engine: "ocr",
        readiness: { state: "ready" },
        profileRevision: 4,
        workerState: "idle",
        installedVersion: "3.0.0",
        modelIdentity: "PP-OCRv5",
        deviceSummary: "cpu",
        lastCheckedAt: "2026-08-22T01:02:03Z",
      },
      {
        engine: "stt",
        readiness: { state: "unavailable", code: "MODEL_NOT_FOUND" },
        profileRevision: 4,
        workerState: "quarantined",
        installedVersion: null,
        modelIdentity: null,
        deviceSummary: null,
        lastCheckedAt: null,
      },
      {
        engine: "tts",
        readiness: { state: "unconfigured" },
        profileRevision: 4,
        workerState: "stopped",
        installedVersion: null,
        modelIdentity: null,
        deviceSummary: null,
        lastCheckedAt: null,
      },
    ],
    ...overrides,
  };
}

function install(overrides: Partial<LocalMediaService> = {}) {
  const service: LocalMediaService = {
    isAvailable: vi.fn(async () => true),
    getProfile: vi.fn(async () => profile()),
    saveProfile: vi.fn(async (input) => ({ ...input.profile, revision: input.profile.revision + 1 })),
    validateProfile: vi.fn(async () => [] as ProfileFieldIssue[]),
    getStatus: vi.fn(async () => status()),
    listAudioDevices: vi.fn(async () => ({
      inputs: [{ deviceId: "mic-1", label: "Built-in Microphone", isDefault: true }],
      outputs: [{ deviceId: "out-1", label: "Speakers", isDefault: true }],
    })),
    probeEngine: vi.fn(async () => ({
      operationId: "op-1",
      kind: "local-media.probe" as const,
      acceptedAt: "2026-08-22T00:00:00Z",
    })),
    selectProfilePath: vi.fn(async () => "/picked/path"),
    selectAndStageOcrSource: vi.fn(),
    discardStagedOcrSource: vi.fn(),
    startOcr: vi.fn(),
    startRecording: vi.fn(),
    stopRecordingAndTranscribe: vi.fn(),
    cancelRecording: vi.fn(),
    startTts: vi.fn(),
    stopPlayback: vi.fn(),
    cancelOperation: vi.fn(),
    getOperationResult: vi.fn(async () => null),
    ...overrides,
  };
  serviceRef.current = service;
  return service;
}

function renderPage() {
  return renderWithAppProviders(
    <LocalMediaPage isActive navigationTarget={null} onNavigate={vi.fn()} searchTerm="" />,
  );
}

async function whenLoaded() {
  await screen.findByTestId("local-media-card-ocr");
}

describe("LocalMediaPage", () => {
  beforeEach(async () => {
    invokeMock.mockClear();
    await activateAppLanguage("zh-CN");
  });

  it("renders one independent card per engine", async () => {
    install();
    renderPage();
    await whenLoaded();

    expect(screen.getByTestId("local-media-card-ocr")).toBeTruthy();
    expect(screen.getByTestId("local-media-card-stt")).toBeTruthy();
    expect(screen.getByTestId("local-media-card-tts")).toBeTruthy();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("shows each engine's own readiness so one failure does not gate the others", async () => {
    install();
    renderPage();
    await whenLoaded();

    expect(screen.getByTestId("local-media-card-ocr").textContent).toContain("可用");
    expect(screen.getByTestId("local-media-card-stt").textContent).toContain("不可用");
    expect(screen.getByTestId("local-media-card-tts").textContent).toContain("待配置");
  });

  it("renders an unavailable engine's stable code as localized copy", async () => {
    install();
    renderPage();
    await whenLoaded();

    expect(screen.getByTestId("local-media-card-stt").textContent).toContain(
      "找不到配置的模型文件或目录",
    );
  });

  it("explains a quarantined worker instead of retrying it silently", async () => {
    install();
    renderPage();
    await whenLoaded();

    expect(screen.getByTestId("local-media-card-stt").textContent).toContain("已暂停自动重启");
  });

  it("shows the probe's version, model, and device as reported", async () => {
    install();
    renderPage();
    await whenLoaded();

    const card = screen.getByTestId("local-media-card-ocr").textContent ?? "";
    expect(card).toContain("3.0.0");
    expect(card).toContain("PP-OCRv5");
  });

  it("offers no install or download control anywhere on the page", async () => {
    install();
    renderPage();
    await whenLoaded();

    const labels = screen.getAllByRole("button").map((button) => button.textContent ?? "");
    expect(labels.some((label) => /下载|安装|Download|Install/.test(label))).toBe(false);
  });

  it("states the local-only guarantee as a property of this feature", async () => {
    install();
    renderPage();
    await whenLoaded();

    expect(screen.getByText(/不会离开本机/)).toBeTruthy();
  });

  it("keeps Save inert until something actually changed", async () => {
    install();
    const { user } = renderPage();
    await whenLoaded();

    const save = screen.getByTestId("local-media-save");
    expect(save.hasAttribute("disabled")).toBe(true);
    await user.clear(screen.getByLabelText("语言", { selector: "#local-media-ocr-language" }));
    await waitFor(() => expect(save.hasAttribute("disabled")).toBe(false));
  });

  it("restores the saved values when the edits are discarded", async () => {
    install();
    const { user } = renderPage();
    await whenLoaded();

    const language = screen.getByLabelText("语言", {
      selector: "#local-media-ocr-language",
    }) as HTMLInputElement;
    await user.clear(language);
    await user.type(language, "en");
    expect(language.value).toBe("en");

    await user.click(screen.getByRole("button", { name: "放弃修改" }));
    await waitFor(() => expect(language.value).toBe("ch"));
  });

  it("blocks a check while there are unsaved edits, because probes run against the saved profile", async () => {
    const service = install();
    const { user } = renderPage();
    await whenLoaded();

    const probe = screen.getByTestId("local-media-probe-ocr");
    expect(probe.hasAttribute("disabled")).toBe(false);
    await user.type(
      screen.getByLabelText("语言", { selector: "#local-media-ocr-language" }),
      "x",
    );

    await waitFor(() => expect(probe.hasAttribute("disabled")).toBe(true));
    expect(service.probeEngine).not.toHaveBeenCalled();
  });

  it("adopts the status a completed probe returns", async () => {
    const probed = status({ profileRevision: 9 });
    probed.engines[2] = { ...probed.engines[2], readiness: { state: "ready" }, installedVersion: "1.9.0" };
    const service = install({
      getOperationResult: vi.fn(async () => ({ kind: "probe" as const, result: probed })),
    });
    const { user } = renderPage();
    await whenLoaded();

    await user.click(screen.getByTestId("local-media-probe-tts"));

    await waitFor(() =>
      expect(screen.getByTestId("local-media-card-tts").textContent).toContain("1.9.0"),
    );
    expect(service.probeEngine).toHaveBeenCalledWith("tts");
  });

  it("marks the offending input when native validation rejects a field", async () => {
    const service = install({
      validateProfile: vi.fn(async () => [
        {
          engine: "ocr" as const,
          field: "pythonExecutable",
          code: "PYTHON_NOT_FOUND" as const,
          messageKey: "localMedia.errors.pythonNotFound",
        },
      ]),
    });
    const { user } = renderPage();
    await whenLoaded();

    await user.type(
      screen.getByLabelText("语言", { selector: "#local-media-ocr-language" }),
      "x",
    );
    await user.click(screen.getByTestId("local-media-save"));

    // Twice on purpose: once beside the input that has to change, and once at the top so the
    // reason a save did not happen is visible without scrolling to find the marked field.
    await waitFor(() =>
      expect(screen.getAllByText("找不到配置的 Python 解释器")).toHaveLength(2),
    );
    const python = screen.getByLabelText("Python 解释器", {
      selector: "#local-media-ocr-python",
    });
    expect(python.getAttribute("aria-invalid")).toBe("true");
    // The save never reached storage, so nothing partially applied.
    expect(service.saveProfile).not.toHaveBeenCalled();
  });

  it("saves with the revision it loaded and reports success", async () => {
    const service = install();
    const { user } = renderPage();
    await whenLoaded();

    await user.type(
      screen.getByLabelText("语言", { selector: "#local-media-ocr-language" }),
      "x",
    );
    await user.click(screen.getByTestId("local-media-save"));

    await waitFor(() => expect(screen.getByText("已保存")).toBeTruthy());
    expect(vi.mocked(service.saveProfile).mock.calls[0][0].expectedRevision).toBe(4);
  });

  it("offers a reload rather than overwriting when the profile changed elsewhere", async () => {
    install({
      saveProfile: vi.fn(async () => {
        throw new Error("PROFILE_REVISION_CONFLICT");
      }),
    });
    const { user } = renderPage();
    await whenLoaded();

    await user.type(
      screen.getByLabelText("语言", { selector: "#local-media-ocr-language" }),
      "x",
    );
    await user.click(screen.getByTestId("local-media-save"));

    await waitFor(() => expect(screen.getByRole("button", { name: "重新载入" })).toBeTruthy());
    expect(screen.getByText(/配置已被其他窗口修改/)).toBeTruthy();
  });

  it("reveals the kokoro voices field only for a kokoro model", async () => {
    install({
      getProfile: vi.fn(async () =>
        profile({
          tts: { ...profile().tts, enabled: true, modelKind: "kokoro" },
        }),
      ),
    });
    renderPage();
    await whenLoaded();

    expect(screen.getByLabelText("voices 文件")).toBeTruthy();
    expect(screen.queryByLabelText("声码器文件")).toBeNull();
  });

  it("lists host audio devices without translating their names", async () => {
    install({
      getProfile: vi.fn(async () => profile({ stt: { ...profile().stt, enabled: true } })),
    });
    renderPage();
    await whenLoaded();

    const microphone = screen.getByLabelText("麦克风", {
      selector: "#local-media-stt-microphone",
    });
    const options = [...microphone.querySelectorAll("option")].map((option) => option.textContent);
    // The host's own wording, verbatim. Only the "no explicit choice" entry is ours to translate.
    expect(options).toEqual(["系统默认设备", "Built-in Microphone"]);
  });

  it("says the page is read-only outside the desktop client", async () => {
    install({
      isAvailable: vi.fn(async () => false),
      getStatus: vi.fn(async () => status({ nativeAvailable: false, platformSupport: "unsupported" })),
    });
    renderPage();
    await whenLoaded();

    expect(screen.getByText(/这些能力需要桌面客户端/)).toBeTruthy();
    expect(screen.getByTestId("local-media-probe-ocr").hasAttribute("disabled")).toBe(true);
  });
});
