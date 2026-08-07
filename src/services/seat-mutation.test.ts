import { describe, expect, it } from "vitest";
import { addSeat, removeSeat } from "./seat-mutation";

const seats = [
  { agentId: "claude-code", roleId: "builtin-architect" },
  { agentId: "codex-cli", roleId: "builtin-reviewer" },
];

describe("addSeat", () => {
  it("appends the seat, keeping the existing order", () => {
    const result = addSeat(seats, { agentId: "gemini-cli", roleId: null });
    expect(result.seats.map((seat) => seat.agentId)).toEqual([
      "claude-code",
      "codex-cli",
      "gemini-cli",
    ]);
  });

  // A one-seat session growing into a multi-seat one must not require recreating it.
  it("grows a single-seat session without recreating it", () => {
    const result = addSeat([{ agentId: "claude-code", roleId: null }], {
      agentId: "codex-cli",
      roleId: "builtin-reviewer",
    });
    expect(result.seats).toHaveLength(2);
    expect(result.agentId).toBe("claude-code");
  });

  it("keeps the mirrored agent id on the first seat", () => {
    expect(addSeat(seats, { agentId: "gemini-cli", roleId: null }).agentId).toBe("claude-code");
  });
});

describe("removeSeat", () => {
  it("removes the seat at the given index", () => {
    const result = removeSeat(seats, 1);
    expect(result?.seats.map((seat) => seat.agentId)).toEqual(["claude-code"]);
  });

  // Removing the first seat changes which Agent the session's mirrored id points at.
  it("re-mirrors the agent id when the first seat is removed", () => {
    const result = removeSeat(seats, 0);
    expect(result?.agentId).toBe("codex-cli");
    expect(result?.seats.map((seat) => seat.agentId)).toEqual(["codex-cli"]);
  });

  // A session always has someone in it; the caller must not be able to empty it.
  it("refuses to remove the last remaining seat", () => {
    expect(removeSeat([{ agentId: "claude-code", roleId: null }], 0)).toBeNull();
  });

  it("returns null for an index that does not exist", () => {
    expect(removeSeat(seats, 5)).toBeNull();
  });
});
