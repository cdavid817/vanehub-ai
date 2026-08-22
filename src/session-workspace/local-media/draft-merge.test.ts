import { describe, expect, it } from "vitest";

import { appendOcrText, appendSpeechTranscript, speechSourceText } from "./draft-merge";

describe("appendSpeechTranscript", () => {
  it("becomes the draft when there was none", () => {
    expect(appendSpeechTranscript("", "你好世界")).toBe("你好世界");
  });

  it("inserts exactly one space after text that does not end in whitespace", () => {
    expect(appendSpeechTranscript("请帮我", "查一下天气")).toBe("请帮我 查一下天气");
  });

  it("adds no separator when the draft already ends in whitespace", () => {
    expect(appendSpeechTranscript("请帮我 ", "查一下")).toBe("请帮我 查一下");
    expect(appendSpeechTranscript("请帮我\n", "查一下")).toBe("请帮我\n查一下");
    expect(appendSpeechTranscript("请帮我\t", "查一下")).toBe("请帮我\t查一下");
  });

  it("leaves the draft untouched for an empty transcript", () => {
    // `NO_SPEECH_DETECTED` reaches here as an empty string; the user's draft must not gain a space.
    expect(appendSpeechTranscript("原文", "")).toBe("原文");
    expect(appendSpeechTranscript("原文", "   ")).toBe("原文");
    expect(appendSpeechTranscript("原文", "\n\n")).toBe("原文");
  });

  it("normalizes line endings without touching the middle of the transcript", () => {
    expect(appendSpeechTranscript("", "第一行\r\n第二行\r第三行")).toBe(
      "第一行\n第二行\n第三行",
    );
    expect(appendSpeechTranscript("", "  两个  空格  ")).toBe("两个  空格");
  });

  it("appends rather than prepends or replaces", () => {
    const result = appendSpeechTranscript("已有内容", "新内容");
    expect(result.startsWith("已有内容")).toBe(true);
    expect(result).toContain("新内容");
  });

  it("preserves text the user typed while transcription was running", () => {
    // The controller re-reads the draft at completion; this asserts the merge itself keeps it.
    const latest = "我在等待时又输入了这些";
    expect(appendSpeechTranscript(latest, "转写结果")).toBe(`${latest} 转写结果`);
  });

  it("handles astral-plane characters without splitting them", () => {
    expect(appendSpeechTranscript("emoji", "🎧🎙️")).toBe("emoji 🎧🎙️");
  });
});

describe("appendOcrText", () => {
  it("becomes the draft when there was none", () => {
    expect(appendOcrText("", "识别文本")).toBe("识别文本");
  });

  it("inserts a blank line before a block appended to existing text", () => {
    expect(appendOcrText("说明：", "识别文本")).toBe("说明：\n\n识别文本");
  });

  it("adds no separator when the draft already ends with a newline", () => {
    expect(appendOcrText("说明：\n", "识别文本")).toBe("说明：\n识别文本");
  });

  it("treats a trailing space as needing a blank line, unlike speech", () => {
    // A space is a sentence continuation; OCR output is a block, so it still gets its own break.
    expect(appendOcrText("说明： ", "识别文本")).toBe("说明： \n\n识别文本");
  });

  it("leaves the draft untouched for empty recognized text", () => {
    expect(appendOcrText("原文", "")).toBe("原文");
    expect(appendOcrText("原文", "   \n  ")).toBe("原文");
  });

  it("keeps internal blank lines from a multi-page result", () => {
    expect(appendOcrText("", "第一页\n\n第二页")).toBe("第一页\n\n第二页");
  });

  it("does not truncate a large block", () => {
    const long = "字".repeat(50_000);
    expect(appendOcrText("", long)).toHaveLength(50_000);
  });
});

describe("speechSourceText", () => {
  it("prefers a non-empty selection", () => {
    expect(speechSourceText("abcdef", { start: 1, end: 4 })).toBe("bcd");
  });

  it("falls back to the whole draft when the selection is collapsed", () => {
    expect(speechSourceText("abcdef", { start: 3, end: 3 })).toBe("abcdef");
    expect(speechSourceText("abcdef", null)).toBe("abcdef");
  });

  it("falls back to the draft when the selection is only whitespace", () => {
    expect(speechSourceText("ab   cd", { start: 2, end: 5 })).toBe("ab   cd");
  });

  it("returns null when there is nothing to read", () => {
    expect(speechSourceText("", null)).toBeNull();
    expect(speechSourceText("   ", null)).toBeNull();
    expect(speechSourceText("   ", { start: 0, end: 3 })).toBeNull();
  });

  it("keeps the selection exactly, without trimming inside it", () => {
    expect(speechSourceText("前 中间文本 后", { start: 2, end: 6 })).toBe("中间文本");
  });
});
