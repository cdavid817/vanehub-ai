// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { FolderOpenerAvailability, FolderOpenerPreferences } from "../../types/folder-opener";

const listFolderOpeners = vi.fn<() => Promise<FolderOpenerAvailability[]>>();
const getFolderOpenerPreferences = vi.fn<() => Promise<FolderOpenerPreferences>>();

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    listFolderOpeners: () => listFolderOpeners(),
    refreshFolderOpeners: () => listFolderOpeners(),
    getFolderOpenerPreferences: () => getFolderOpenerPreferences(),
    saveFolderOpenerPreferences: vi.fn(),
  },
}));

const { FolderOpenersSection } = await import("./folder-openers-section");

function opener(id: FolderOpenerAvailability["id"], status: FolderOpenerAvailability["status"], executablePath: string | null = null): FolderOpenerAvailability {
  return { id, category: "ide", status, executablePath, version: null, edition: null, detectionSource: null, iconKey: id, reason: null };
}

describe("FolderOpenersSection", () => {
  beforeEach(() => {
    listFolderOpeners.mockReset();
    getFolderOpenerPreferences.mockReset();
  });

  it("shows a disabled empty state instead of a blank select when nothing is available", async () => {
    // A Linux host before cross-platform discovery: every opener is missing or unsupported.
    listFolderOpeners.mockResolvedValue([
      opener("file-explorer", "not-installed"),
      opener("intellij-idea", "not-installed"),
      opener("windows-terminal", "unsupported-platform"),
    ]);
    getFolderOpenerPreferences.mockResolvedValue({ configuredDefaultOpenerId: "file-explorer", effectiveDefaultOpenerId: null, enabledOpenerIds: ["file-explorer"], fallbackActive: true });

    render(<FolderOpenersSection />);

    const select = await screen.findByRole("combobox", { name: "默认打开方式" });
    expect((select as HTMLSelectElement).disabled).toBe(true);
    expect(await screen.findByRole("option", { name: "未检测到可用程序" })).toBeTruthy();
  });

  it("presents availability as a status pill with the resolved launcher path", async () => {
    listFolderOpeners.mockResolvedValue([
      opener("file-explorer", "available", "/usr/bin/xdg-open"),
      { ...opener("intellij-idea", "available", "/home/me/.local/share/JetBrains/Toolbox/apps/intellij-idea-ultimate/bin/idea.sh"), version: "2025.1", edition: "Ultimate" },
      opener("git-bash", "unsupported-platform"),
    ]);
    getFolderOpenerPreferences.mockResolvedValue({ configuredDefaultOpenerId: "intellij-idea", effectiveDefaultOpenerId: "intellij-idea", enabledOpenerIds: ["intellij-idea", "file-explorer"], fallbackActive: false });

    render(<FolderOpenersSection />);

    const select = await screen.findByRole("combobox", { name: "默认打开方式" });
    expect((select as HTMLSelectElement).value).toBe("intellij-idea");
    expect((select as HTMLSelectElement).disabled).toBe(false);
    expect(screen.getAllByText("已安装")).toHaveLength(2);
    expect(screen.getByText("当前平台不支持")).toBeTruthy();
    expect(screen.getByText("版本: 2025.1 · 版本类型: Ultimate")).toBeTruthy();
    expect(screen.getByTitle("/home/me/.local/share/JetBrains/Toolbox/apps/intellij-idea-ultimate/bin/idea.sh")).toBeTruthy();
    expect(screen.queryByRole("option", { name: "Git Bash" })).toBeNull();
  });
});
