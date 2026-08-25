// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { activateAppLanguage } from "../../i18n";
import type {
  LocalMediaComposerModel,
  MicrophonePhase,
} from "../../session-workspace/local-media/local-media-composer-types";
import { renderWithAppProviders } from "../../test/render";
import { ComposerMediaActions } from "./ComposerMediaActions";

function model(overrides: Partial<LocalMediaComposerModel> = {}): LocalMediaComposerModel {
  return {
    availability: { native: true, ocr: true, stt: true, tts: true },
    ocrPhase: "idle",
    microphonePhase: "idle",
    speechPhase: "idle",
    recordingElapsedMs: 0,
    recordingLimitReached: false,
    failure: null,
    review: null,
    overflow: null,
    startOcr: vi.fn(),
    updateReviewText: vi.fn(),
    appendReviewText: vi.fn(),
    cancelReview: vi.fn(),
    microphone: {
      onPointerDown: vi.fn(),
      onPointerUp: vi.fn(),
      onPointerCancel: vi.fn(),
      onLostPointerCapture: vi.fn(),
      onKeyDown: vi.fn(),
      onKeyUp: vi.fn(),
      onClickCapture: vi.fn(),
    },
    toggleSpeech: vi.fn(),
    dismissFailure: vi.fn(),
    dismissOverflow: vi.fn(),
    ...overrides,
  };
}

const OCR = "composer-media-ocr";
const MIC = "composer-media-microphone";
const SPEAK = "composer-media-speak";

function render(overrides: Partial<LocalMediaComposerModel> = {}, hasText = true) {
  const media = model(overrides);
  const view = renderWithAppProviders(<ComposerMediaActions hasText={hasText} media={media} />);
  return { media, ...view };
}

describe("ComposerMediaActions", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("labels every icon-only control", () => {
    render();

    expect(screen.getByTestId(OCR).getAttribute("aria-label")).toBe("图片文字识别");
    expect(screen.getByTestId(MIC).getAttribute("aria-label")).toBe("按住说话");
    expect(screen.getByTestId(SPEAK).getAttribute("aria-label")).toBe("朗读文本");
  });

  it("keeps every control the same size in every state", () => {
    // The toolbar sits directly under the text being typed; a control that changes size when a
    // spinner replaces its icon would move the caret's surroundings mid-sentence.
    const idle = render().getByTestId(OCR).className;
    const busy = render({ ocrPhase: "running" }).getAllByTestId(OCR)[1].className;
    expect(idle).toBe(busy);
  });

  it("disables an action whose own engine is unready and leaves the others alone", () => {
    render({ availability: { native: true, ocr: false, stt: true, tts: true } });

    expect(screen.getByTestId(OCR).hasAttribute("disabled")).toBe(true);
    expect(screen.getByTestId(MIC).hasAttribute("disabled")).toBe(false);
    expect(screen.getByTestId(SPEAK).hasAttribute("disabled")).toBe(false);
  });

  it("explains a disabled control as native-only when there is no native runtime", () => {
    render({ availability: { native: false, ocr: false, stt: false, tts: false } });

    expect(screen.getByTestId(OCR).getAttribute("title")).toBe("该功能只在桌面客户端可用");
  });

  it("explains a disabled control as unready when the runtime is present", () => {
    render({ availability: { native: true, ocr: false, stt: false, tts: false } });

    expect(screen.getByTestId(OCR).getAttribute("title")).toContain("尚未就绪");
  });

  it("disables read-aloud when there is nothing to read", () => {
    render({}, false);

    expect(screen.getByTestId(SPEAK).hasAttribute("disabled")).toBe(true);
  });

  it("keeps read-aloud pressable while speaking so it can be stopped", () => {
    render({ speechPhase: "playing" }, false);

    const speak = screen.getByTestId(SPEAK);
    expect(speak.hasAttribute("disabled")).toBe(false);
    expect(speak.getAttribute("aria-pressed")).toBe("true");
    expect(speak.getAttribute("aria-label")).toBe("停止朗读");
  });

  it.each<[MicrophonePhase, string]>([
    ["opening", "true"],
    ["recording", "true"],
    ["idle", "false"],
  ])("reports the hold state as aria-pressed during %s", (microphonePhase, pressed) => {
    render({ microphonePhase });

    expect(screen.getByTestId(MIC).getAttribute("aria-pressed")).toBe(pressed);
  });

  it("blocks a new hold while the previous utterance is still transcribing", () => {
    render({ microphonePhase: "transcribing" });

    expect(screen.getByTestId(MIC).hasAttribute("disabled")).toBe(true);
  });

  it.each<[MicrophonePhase, string]>([
    ["opening", "正在打开麦克风"],
    ["recording", "录音中"],
    ["finalizing", "正在结束录音"],
    ["transcribing", "正在转写"],
  ])("announces the %s phase in one polite live region", (microphonePhase, text) => {
    render({ microphonePhase });

    const status = screen.getByTestId("composer-media-status");
    expect(status.textContent).toBe(text);
    expect(status.getAttribute("aria-live")).toBe("polite");
  });

  it("keeps the elapsed timer out of the accessibility tree", () => {
    render({ microphonePhase: "recording", recordingElapsedMs: 65_000 });

    const elapsed = screen.getByTestId("composer-media-elapsed");
    expect(elapsed.textContent).toBe("01:05");
    // A timer that announced every second would drown out everything else a screen reader says.
    expect(elapsed.getAttribute("aria-hidden")).toBe("true");
  });

  it("respects reduced motion for every spinner", () => {
    render({ ocrPhase: "running", microphonePhase: "transcribing", speechPhase: "generating" });

    for (const id of [OCR, MIC, SPEAK]) {
      const spinner = screen.getByTestId(id).querySelector("svg");
      expect(spinner?.getAttribute("class")).toContain("motion-safe:animate-spin");
    }
  });

  it("announces that the duration ceiling was reached even after recording stops", () => {
    render({ microphonePhase: "idle", recordingLimitReached: true });

    expect(screen.getByTestId("composer-media-status").textContent).toContain("录音时长上限");
  });

  it("shows a failure inline with its localized message and a dismiss control", async () => {
    const { media, user } = render({
      failure: { engine: "stt", code: "MIC_PERMISSION_DENIED" },
    });

    const failure = screen.getByTestId("composer-media-failure");
    expect(failure.textContent).toContain("系统未授予麦克风权限");
    expect(failure.getAttribute("role")).toBe("alert");
    await user.click(screen.getByRole("button", { name: "关闭提示" }));
    expect(media.dismissFailure).toHaveBeenCalledTimes(1);
  });

  it("routes the pick action through the controller rather than any dialog of its own", async () => {
    const { media, user } = render();

    await user.click(screen.getByTestId(OCR));
    expect(media.startOcr).toHaveBeenCalledTimes(1);
  });
});
