import { describe, expect, it } from "vitest";
import { normalizeToolUse, upsertToolUse } from "./tool-use";

describe("tool-use reconciliation", () => {
  it("appends a new stable id", () => {
    expect(upsertToolUse([], { id: "call-1", name: "shell", status: "running" })).toEqual([
      { id: "call-1", name: "shell", status: "running" },
    ]);
  });

  it("updates an existing call without losing omitted input or output", () => {
    const current = [{
      id: "call-1",
      name: "shell",
      input: { command: "npm test" },
      output: "partial",
      status: "running" as const,
    }];

    expect(upsertToolUse(current, { id: "call-1", name: "shell", status: "completed" })).toEqual([
      {
        id: "call-1",
        name: "shell",
        input: { command: "npm test" },
        output: "partial",
        status: "completed",
      },
    ]);
  });

  it("normalizes persisted duplicate status snapshots in first-seen order", () => {
    expect(normalizeToolUse([
      { id: "shell-1", name: "shell", input: { command: "git status" }, status: "running" },
      { id: "file-1", name: "file", input: { path: "README.md" }, status: "completed" },
      { id: "shell-1", name: "shell", output: "clean", status: "completed" },
    ])).toEqual([
      {
        id: "shell-1",
        name: "shell",
        input: { command: "git status" },
        output: "clean",
        status: "completed",
      },
      { id: "file-1", name: "file", input: { path: "README.md" }, status: "completed" },
    ]);
  });
});
