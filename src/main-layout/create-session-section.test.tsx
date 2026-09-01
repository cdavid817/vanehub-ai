// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { FolderTree } from "lucide-react";
import { describe, expect, it } from "vitest";
import { CreateSessionSection } from "./create-session-section";

describe("CreateSessionSection's mount focus (task 11.14's 'error focus')", () => {
  it("focuses its own heading as soon as it mounts", () => {
    render(
      <CreateSessionSection hint="hint text" icon={FolderTree} title="Step title">
        <p>content</p>
      </CreateSessionSection>,
    );
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Step title" }));
  });

  it("re-focuses a fresh heading when the wizard remounts a different step", () => {
    const { rerender } = render(
      <CreateSessionSection hint="hint text" icon={FolderTree} title="Step one">
        <p>content</p>
      </CreateSessionSection>,
    );
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Step one" }));

    // The wizard conditionally renders one step at a time (`wizard.step === N ? <StepN/> : null`),
    // so a transition is an unmount of the old step and a fresh mount of the new one -- modeled
    // here by unmounting first, the same as the real conditional does.
    rerender(<></>);
    rerender(
      <CreateSessionSection hint="hint text" icon={FolderTree} title="Step two">
        <p>content</p>
      </CreateSessionSection>,
    );
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "Step two" }));
  });
});
