import { describe, expect, it } from "vitest";
import tauriConfig from "../src-tauri/tauri.conf.json";

describe("desktop main window configuration", () => {
  it("starts maximized without entering fullscreen", () => {
    const mainWindow = tauriConfig.app.windows[0];

    expect(mainWindow.maximized).toBe(true);
    expect(mainWindow.fullscreen).toBe(false);
    expect(mainWindow.width).toBe(1280);
    expect(mainWindow.height).toBe(820);
    expect(mainWindow.minWidth).toBe(1100);
    expect(mainWindow.minHeight).toBe(700);
  });
});
