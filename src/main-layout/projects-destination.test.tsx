// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProjectsDestination } from "./projects-destination";

vi.mock("../components/lazy-feature", () => ({
  LazyFeature: ({ componentProps }: { componentProps: { onOpenSshSettings: () => void } }) => (
    <button data-testid="lazy-feature" onClick={componentProps.onOpenSshSettings} type="button" />
  ),
}));

describe("ProjectsDestination", () => {
  it("renders a single lazy-loaded panel and no secondary navigation", () => {
    render(
      <ProjectsDestination
        onContinueSession={vi.fn()}
        onNewSessionForWorkspace={vi.fn()}
        onOpenSettings={vi.fn()}
      />,
    );
    expect(screen.getByTestId("lazy-feature")).toBeTruthy();
    expect(screen.queryByRole("tablist")).toBeNull();
    expect(screen.queryByRole("tab")).toBeNull();
  });

  it("pre-binds onOpenSshSettings to the ssh-connections settings page rather than forwarding a raw pageId", () => {
    const onOpenSettings = vi.fn();
    render(
      <ProjectsDestination
        onContinueSession={vi.fn()}
        onNewSessionForWorkspace={vi.fn()}
        onOpenSettings={onOpenSettings}
      />,
    );

    fireEvent.click(screen.getByTestId("lazy-feature"));

    expect(onOpenSettings).toHaveBeenCalledWith("ssh-connections");
  });
});
