// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { QuestionCard, parseQuestionPrompt } from "./QuestionCard";

const { agentService } = vi.hoisted(() => ({
  agentService: { resolveAgentQuestion: vi.fn() },
}));
vi.mock("../../services/runtime-agent-client", () => ({ agentService }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const prompt = { question: "Which approach should I take?", options: ["Rewrite it", "Patch it"] };

describe("parseQuestionPrompt", () => {
  it("reads the question and options straight off the tool input", () => {
    expect(parseQuestionPrompt(prompt)).toEqual(prompt);
  });

  it("drops non-string and blank options rather than rendering them", () => {
    expect(parseQuestionPrompt({ question: "Which?", options: ["keep", "", 7, "  "] })).toEqual({
      question: "Which?",
      options: ["keep"],
    });
  });

  it("returns null for input that is not a question", () => {
    for (const input of [null, undefined, "text", [], {}, { question: "Which?" }, { question: "  ", options: ["a"] }]) {
      expect(parseQuestionPrompt(input)).toBeNull();
    }
  });
});

describe("QuestionCard", () => {
  beforeEach(() => {
    agentService.resolveAgentQuestion.mockReset().mockResolvedValue(true);
  });

  it("submits the option the user picked", async () => {
    render(<QuestionCard callId="call-1" input={prompt} sessionId="session-1" />);

    await userEvent.click(screen.getByRole("button", { name: "Patch it" }));

    await waitFor(() =>
      expect(agentService.resolveAgentQuestion).toHaveBeenCalledWith("session-1", "call-1", "Patch it"),
    );
  });

  // The offered options are the model's guess at the answer space, and an incomplete guess is
  // exactly the situation that made asking worthwhile.
  it("submits free text that matches none of the offered options", async () => {
    render(<QuestionCard callId="call-1" input={prompt} sessionId="session-1" />);

    await userEvent.type(screen.getByLabelText("chat.toolQuestion.otherLabel"), "neither, do the third thing");
    await userEvent.click(screen.getByRole("button", { name: "chat.toolQuestion.submit" }));

    await waitFor(() =>
      expect(agentService.resolveAgentQuestion).toHaveBeenCalledWith(
        "session-1",
        "call-1",
        "neither, do the third thing",
      ),
    );
  });

  it("does not submit whitespace-only free text", async () => {
    render(<QuestionCard callId="call-1" input={prompt} sessionId="session-1" />);

    await userEvent.type(screen.getByLabelText("chat.toolQuestion.otherLabel"), "   ");
    const submit = screen.getByRole("button", { name: "chat.toolQuestion.submit" }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);
    expect(agentService.resolveAgentQuestion).not.toHaveBeenCalled();
  });

  it("renders nothing when the tool input is not a question", () => {
    const { container } = render(<QuestionCard callId="call-1" input={{ command: "ls" }} sessionId="session-1" />);
    expect(container.innerHTML).toBe("");
  });
});
