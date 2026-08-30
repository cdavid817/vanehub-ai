// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { Settings } from "lucide-react";
import { describe, expect, it } from "vitest";
import { FormSection } from "./FormSection";

describe("FormSection", () => {
  it("renders title, description, and children", () => {
    render(
      <FormSection description="Core preferences" title="Basic">
        <p>Field content</p>
      </FormSection>,
    );
    expect(screen.getByText("Basic")).toBeTruthy();
    expect(screen.getByText("Core preferences")).toBeTruthy();
    expect(screen.getByText("Field content")).toBeTruthy();
  });

  it("renders an optional icon and header actions", () => {
    const { container } = render(
      <FormSection actions={<button type="button">Reset</button>} icon={Settings} title="Basic">
        <p>Field content</p>
      </FormSection>,
    );
    expect(container.querySelector("svg")).not.toBeNull();
    expect(screen.getByRole("button", { name: "Reset" })).toBeTruthy();
  });
});
