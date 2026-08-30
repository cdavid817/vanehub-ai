// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { FieldError } from "./FieldError";

describe("FieldError", () => {
  it("renders nothing when there is no message", () => {
    const { container } = render(<FieldError />);
    expect(container.firstChild).toBeNull();
  });

  it("renders the message as an alert, wired for aria-describedby", () => {
    render(<FieldError id="model-error" message="Model is required." />);
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Model is required.");
    expect(alert.id).toBe("model-error");
  });
});
