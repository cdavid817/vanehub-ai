// @vitest-environment jsdom

import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { RuntimePanel, type RuntimePanelTab } from "./RuntimePanel";

function StatefulTab({ id }: { id: string }) {
  const [count, setCount] = useState(0);
  return (
    <div>
      <span>{id} count: {count}</span>
      <button onClick={() => setCount((value) => value + 1)} type="button">Increment {id}</button>
    </div>
  );
}

function ControlledRuntimePanel({ tabs }: { tabs: RuntimePanelTab[] }) {
  const [activeTabId, setActiveTabId] = useState(tabs[0].id);
  const [maximized, setMaximized] = useState(false);
  return (
    <RuntimePanel
      activeTabId={activeTabId}
      maximized={maximized}
      onActiveTabChange={setActiveTabId}
      onClose={vi.fn()}
      onMaximizedChange={setMaximized}
      tabs={tabs}
    />
  );
}

describe("RuntimePanel", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("only mounts a tab once it has actually been activated", () => {
    const shellRender = vi.fn(() => <p>Shell content</p>);
    const logsRender = vi.fn(() => <p>Logs content</p>);
    render(
      <RuntimePanel
        activeTabId="shell"
        maximized={false}
        onActiveTabChange={vi.fn()}
        onClose={vi.fn()}
        onMaximizedChange={vi.fn()}
        tabs={[{ id: "shell", label: "Shell", render: shellRender }, { id: "logs", label: "Logs", render: logsRender }]}
      />,
    );
    expect(shellRender).toHaveBeenCalled();
    expect(logsRender).not.toHaveBeenCalled();
    expect(screen.queryByText("Logs content")).toBeNull();
  });

  it("passes isVisible true only to the active tab", () => {
    render(
      <ControlledRuntimePanel
        tabs={[
          { id: "shell", label: "Shell", render: (visible) => <p>Shell visible: {String(visible)}</p> },
          { id: "logs", label: "Logs", render: (visible) => <p>Logs visible: {String(visible)}</p> },
        ]}
      />,
    );
    expect(screen.getByText("Shell visible: true")).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: "Logs" }));
    expect(screen.getByText("Shell visible: false")).toBeTruthy();
    expect(screen.getByText("Logs visible: true")).toBeTruthy();
  });

  it("keeps a previously opened tab's component state after switching away and back", () => {
    render(
      <ControlledRuntimePanel
        tabs={[
          { id: "shell", label: "Shell", render: () => <StatefulTab id="shell" /> },
          { id: "logs", label: "Logs", render: () => <StatefulTab id="logs" /> },
        ]}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Increment shell" }));
    fireEvent.click(screen.getByRole("button", { name: "Increment shell" }));
    expect(screen.getByText("shell count: 2")).toBeTruthy();

    fireEvent.click(screen.getByRole("tab", { name: "Logs" }));
    fireEvent.click(screen.getByRole("tab", { name: "Shell" }));
    expect(screen.getByText("shell count: 2")).toBeTruthy();
  });

  it("cycles tabs with arrow keys and wraps at the ends", () => {
    render(
      <ControlledRuntimePanel
        tabs={[
          { id: "shell", label: "Shell", render: () => <p>Shell</p> },
          { id: "logs", label: "Logs", render: () => <p>Logs</p> },
        ]}
      />,
    );
    const shellTab = screen.getByRole("tab", { name: "Shell" });
    fireEvent.keyDown(shellTab, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "Logs" }).getAttribute("aria-selected")).toBe("true");
    fireEvent.keyDown(screen.getByRole("tab", { name: "Logs" }), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "Shell" }).getAttribute("aria-selected")).toBe("true");
  });

  it("toggles maximize/restore and calls onClose", () => {
    const onMaximizedChange = vi.fn();
    const onClose = vi.fn();
    render(
      <RuntimePanel
        activeTabId="shell"
        maximized={false}
        onActiveTabChange={vi.fn()}
        onClose={onClose}
        onMaximizedChange={onMaximizedChange}
        tabs={[{ id: "shell", label: "Shell", render: () => <p>Shell</p> }]}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Maximize" }));
    expect(onMaximizedChange).toHaveBeenCalledWith(true);
    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
