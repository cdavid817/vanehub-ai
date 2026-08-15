// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { FilesTab } from "./files-tab";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    listSessionDirectory: vi.fn(),
    readSessionFile: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: mockAgentService,
}));

describe("FilesTab", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("surfaces an error in the tree section when a directory expand fails", async () => {
    mockAgentService.listSessionDirectory.mockImplementation((_sessionId: string, path: string) => {
      if (path === "") {
        return Promise.resolve({
          items: [{ name: "src", path: "src", kind: "directory", size: null }],
          truncated: false,
          nextCursor: null,
          context: { availability: "available" as const, rootName: "project", reason: null },
          path: "",
        });
      }
      return Promise.reject(new Error("Permission denied"));
    });

    const { user } = renderWithAppProviders(<FilesTab sessionId="session-1" />);

    await waitFor(() => {
      expect(screen.getByText("src")).toBeTruthy();
    });

    await user.click(screen.getByText("src"));

    // The tree section is the first <section> in the split layout.
    // Currently the error is only rendered in the preview panel (second section),
    // so this assertion MUST fail — proving the bug exists.
    await waitFor(() => {
      const sections = document.querySelectorAll("section");
      expect(sections.length).toBeGreaterThanOrEqual(1);
      expect(sections[0].textContent?.toLowerCase()).toContain("could not be completed");
    });
  });

  it("makes files draggable and copyable but not directories", async () => {
    mockAgentService.listSessionDirectory.mockResolvedValue({
      items: [
        { name: "src", path: "src", kind: "directory", size: null },
        { name: "main.rs", path: "main.rs", kind: "file", size: 12 },
      ],
      truncated: false,
      nextCursor: null,
      context: { availability: "available" as const, rootName: "project", reason: null },
      path: "",
    });

    renderWithAppProviders(<FilesTab sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("main.rs")).toBeTruthy());

    const fileRow = screen.getByText("main.rs").closest("button");
    const directoryRow = screen.getByText("src").closest("button");
    expect(fileRow?.getAttribute("draggable")).toBe("true");
    // A directory is not referenceable, so it must not start a drag.
    expect(directoryRow?.getAttribute("draggable")).toBe("false");

    expect(screen.getByTestId("copy-path-main.rs")).toBeTruthy();
    expect(screen.queryByTestId("copy-path-src")).toBeNull();
  });

  it("does not expand a directory when loading fails", async () => {
    mockAgentService.listSessionDirectory.mockImplementation((_sessionId: string, path: string) => {
      if (path === "") {
        return Promise.resolve({
          items: [{ name: "src", path: "src", kind: "directory", size: null }],
          truncated: false,
          nextCursor: null,
          context: { availability: "available" as const, rootName: "project", reason: null },
          path: "",
        });
      }
      return Promise.reject(new Error("Permission denied"));
    });

    const { user } = renderWithAppProviders(<FilesTab sessionId="session-1" />);

    await waitFor(() => {
      expect(screen.getByText("src")).toBeTruthy();
    });

    await user.click(screen.getByText("src"));

    // The directory should stay collapsed (ChevronRight) when loading fails,
    // not appear expanded (ChevronDown) with no children.
    await waitFor(() => {
      const treeSection = document.querySelectorAll("section")[0];
      expect(treeSection.querySelector(".lucide-chevron-right")).toBeTruthy();
      expect(treeSection.querySelector(".lucide-chevron-down")).toBeFalsy();
    });
  });
});
