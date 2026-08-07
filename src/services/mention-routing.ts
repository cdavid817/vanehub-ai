export type HandoffTruncationReason = "too-many-mentions";

export interface HandoffMentions {
  targets: string[];
  /** Why some mentions were dropped, so the chain's end can be explained rather than silent. */
  truncatedReason: HandoffTruncationReason | null;
}

/** `>` quotes and `-`/`*`/`1.` list markers still count as the start of a line. */
const linePrefix = /^\s*(?:(?:>\s*)|(?:[-*+]\s+)|(?:\d+[.)]\s+))*/;
/** A handle ends at whitespace or punctuation; without this, `@opus-45` would match `@opus`. */
const boundary = /[\s,.:;!?()[\]{}<>，。！？、：；（）【】《》「」『』〈〉]/;

function stripFencedCode(text: string): string {
  // An Agent explaining how routing works will paste an example; that must not dispatch anyone.
  const lines = text.split("\n");
  const kept: string[] = [];
  let inFence = false;
  for (const line of lines) {
    if (line.trimStart().startsWith("```")) {
      inFence = !inFence;
      continue;
    }
    if (!inFence) kept.push(line);
  }
  return kept.join("\n");
}

/**
 * Finds the seats a completed reply hands off to.
 *
 * Only a mention at the start of a line routes. Any-position matching makes ordinary prose
 * unpredictable — writing "让 @代码审查 看一下" should describe an intention, not dispatch someone.
 */
export function parseHandoffMentions({
  maxMentions,
  mentions,
  selfMention,
  text,
}: {
  maxMentions: number;
  mentions: string[];
  selfMention: string | null;
  text: string;
}): HandoffMentions {
  // Longest first, so a handle that prefixes another cannot shadow it.
  const ordered = [...mentions].sort((left, right) => right.length - left.length);
  const found: string[] = [];
  let truncated = false;

  for (const line of stripFencedCode(text).split("\n")) {
    const rest = line.replace(linePrefix, "");
    if (!rest.startsWith("@")) continue;
    const candidate = rest.slice(1);
    const handle = ordered.find(
      (mention) =>
        candidate.startsWith(mention) &&
        (candidate.length === mention.length || boundary.test(candidate[mention.length])),
    );
    if (!handle) continue;
    if (selfMention && handle === selfMention) continue;
    if (found.includes(handle)) continue;
    if (found.length >= maxMentions) {
      truncated = true;
      continue;
    }
    found.push(handle);
  }

  return { targets: found, truncatedReason: truncated ? "too-many-mentions" : null };
}
