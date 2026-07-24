import { describe, expect, it } from "vitest";
import { captureWebGap, captureWebOutput, configureWebCapture, purgeWebCapture, searchWebCapture } from "./web-terminal-capture-client";

describe("web terminal capture", () => {
  it("simulates search, gap, capacity and purge", () => {
    configureWebCapture(32);
    captureWebOutput({ sessionId: "capture-session", connectionId: null, terminalId: "terminal", runId: null, source: "pty", content: "hello" });
    captureWebGap("capture-session");
    expect(searchWebCapture("gap", "capture-session")).toHaveLength(1);
    captureWebOutput({ sessionId: "capture-session", connectionId: null, terminalId: "terminal", runId: null, source: "pty", content: "0123456789" });
    expect(purgeWebCapture("capture-session")).toBeGreaterThan(0);
  });

  it("keeps multilingual output searchable and paginated", () => {
    configureWebCapture(1024);
    purgeWebCapture();
    for (const content of ["路径 /工作区", "构建成功", "東京 terminal"]) captureWebOutput({ sessionId: "cjk-session", connectionId: "ssh", terminalId: "terminal", runId: null, source: "quick-command", content });
    expect(searchWebCapture("工作区", "cjk-session", 0, 1)[0].content).toContain("工作区");
    expect(searchWebCapture("terminal", "cjk-session")).toHaveLength(1);
    expect(searchWebCapture("", "cjk-session", 0, 2)).toHaveLength(2);
  });
});
