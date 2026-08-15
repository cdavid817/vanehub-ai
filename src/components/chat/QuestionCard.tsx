import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../../services/runtime-agent-client";
import { Button } from "../ui/button";

interface QuestionPrompt {
  question: string;
  options: string[];
}

/**
 * Reads the question straight out of the tool call's own input. Unlike `ApprovalCard`, which has
 * to fetch the permission record behind the call, everything a question needs was already sent by
 * the model and is already on the block (`add-agent-user-question` D2).
 */
export function parseQuestionPrompt(input: unknown): QuestionPrompt | null {
  if (!input || typeof input !== "object" || Array.isArray(input)) return null;
  const record = input as Record<string, unknown>;
  const question = typeof record.question === "string" ? record.question.trim() : "";
  const options = Array.isArray(record.options)
    ? record.options.filter((option): option is string => typeof option === "string" && option.trim().length > 0)
    : [];
  if (!question || options.length === 0) return null;
  return { question, options };
}

export function QuestionCard({ sessionId, callId, input }: { sessionId: string; callId: string; input: unknown }) {
  const { t } = useTranslation();
  const prompt = parseQuestionPrompt(input);
  const [freeText, setFreeText] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const freeTextId = useId();

  if (!prompt) return null;

  async function submit(answer: string) {
    const trimmed = answer.trim();
    if (!trimmed || submitting) return;
    setSubmitting(true);
    try {
      await agentService.resolveAgentQuestion(sessionId, callId, trimmed);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex flex-col gap-2 border-t border-primary/30 bg-primary/5 px-3 py-2" data-testid="tool-question">
      <p className="text-xs text-foreground">{prompt.question}</p>
      <div className="flex flex-wrap gap-2">
        {prompt.options.map((option) => (
          <Button key={option} disabled={submitting} onClick={() => void submit(option)} size="sm" variant="outline">
            {option}
          </Button>
        ))}
      </div>
      {/* Always present: the offered options are the model's guess at the answer space, and an
          incomplete guess is exactly the situation that made asking worthwhile. */}
      <div className="flex items-center gap-2">
        <label className="sr-only" htmlFor={freeTextId}>
          {t("chat.toolQuestion.otherLabel")}
        </label>
        <input
          className="min-w-0 flex-1 rounded-md border bg-background px-2 py-1 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          disabled={submitting}
          id={freeTextId}
          onChange={(event) => setFreeText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void submit(freeText);
          }}
          placeholder={t("chat.toolQuestion.otherPlaceholder")}
          value={freeText}
        />
        <Button disabled={submitting || !freeText.trim()} onClick={() => void submit(freeText)} size="sm">
          {t("chat.toolQuestion.submit")}
        </Button>
      </div>
    </div>
  );
}
