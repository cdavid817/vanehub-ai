// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { activateAppLanguage } from "../../i18n";
import type { OcrReviewState } from "../../session-workspace/local-media/local-media-composer-types";
import {
  ocrResult,
  stagedSource,
} from "../../session-workspace/local-media/local-media-test-double";
import { renderWithAppProviders } from "../../test/render";
import { LocalMediaResultDialog } from "./LocalMediaResultDialog";
import { OcrReviewDialog } from "./OcrReviewDialog";

function review(overrides: Partial<OcrReviewState> = {}): OcrReviewState {
  return {
    source: stagedSource(),
    result: ocrResult("line one\nline two"),
    text: "line one\nline two",
    ...overrides,
  };
}

function renderDialog(state = review()) {
  const handlers = { onAppend: vi.fn(), onCancel: vi.fn(), onChange: vi.fn() };
  const view = renderWithAppProviders(<OcrReviewDialog review={state} {...handlers} />);
  return { ...handlers, ...view };
}

describe("OcrReviewDialog", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("shows the source, page count, and engine provenance", () => {
    renderDialog();

    const panel = screen.getByTestId("composer-ocr-review");
    expect(panel.textContent).toContain("invoice.png");
    expect(panel.textContent).toContain("1 页");
    expect(panel.textContent).toContain("paddleocr 3.0.0");
  });

  it("states that recognition stayed on this machine", () => {
    renderDialog();

    expect(screen.getByTestId("composer-ocr-review").textContent).toContain("识别在本机完成");
  });

  it("renders each engine warning from its locale key", () => {
    renderDialog(
      review({
        result: ocrResult("text", {
          warnings: [
            {
              code: "OUTPUT_TRUNCATED",
              messageKey: "localMedia.warnings.outputTruncated",
              pageNumber: 2,
            },
          ],
        }),
      }),
    );

    expect(screen.getByTestId("composer-ocr-review").textContent).toContain("超出长度上限");
  });

  it("reports the character count of the edited text, not the original", async () => {
    const state = review({ text: "abc" });
    renderDialog(state);

    expect(screen.getByTestId("composer-ocr-review").textContent).toContain("3 个字符");
  });

  it("reports every edit so the controller owns the text", async () => {
    const { onChange, user } = renderDialog();

    await user.type(screen.getByTestId("composer-ocr-text"), "!");
    expect(onChange).toHaveBeenCalled();
  });

  it("blocks Append when the recognized text is empty and says why", () => {
    renderDialog(review({ text: "   ", result: ocrResult("   ") }));

    expect(screen.getByTestId("composer-ocr-append").hasAttribute("disabled")).toBe(true);
    expect(screen.getByTestId("composer-ocr-review").textContent).toContain("没有识别到文字");
  });

  it("appends only on the explicit action", async () => {
    const { onAppend, user } = renderDialog();

    expect(onAppend).not.toHaveBeenCalled();
    await user.click(screen.getByTestId("composer-ocr-append"));
    expect(onAppend).toHaveBeenCalledTimes(1);
  });

  it("copies the reviewed text without touching the draft", async () => {
    const { onAppend, onCancel, user } = renderDialog();
    // After `userEvent.setup()`, which installs a clipboard stub of its own during render.
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });

    await user.click(screen.getByTestId("composer-ocr-copy"));

    expect(writeText).toHaveBeenCalledWith("line one\nline two");
    // Copy is an escape hatch, not a second way to append.
    expect(onAppend).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("cancels through the dedicated action", async () => {
    const { onCancel, user } = renderDialog();

    await user.click(screen.getByRole("button", { name: "放弃" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });
});

describe("LocalMediaResultDialog", () => {
  beforeEach(async () => {
    await activateAppLanguage("zh-CN");
  });

  it("keeps an oversized result readable and copyable instead of truncating it", async () => {
    const onClose = vi.fn();
    const { user } = renderWithAppProviders(
      <LocalMediaResultDialog engine="ocr" onClose={onClose} text="a very long result" />,
    );
    // After `userEvent.setup()`, which installs a clipboard stub of its own during render.
    const writeText = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    const textarea = screen.getByTestId("composer-media-overflow") as HTMLTextAreaElement;
    expect(textarea.value).toBe("a very long result");
    expect(textarea.readOnly).toBe(true);
    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(writeText).toHaveBeenCalledWith("a very long result");
  });

  it("titles the dialog for the engine that produced the result", () => {
    renderWithAppProviders(
      <LocalMediaResultDialog engine="stt" onClose={vi.fn()} text="transcript" />,
    );

    expect(screen.getByRole("heading", { name: "转写结果过长，未写入输入框" })).toBeTruthy();
  });
});
