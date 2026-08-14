export type ParsedInput =
  | { kind: "message" }
  | { kind: "literal"; content: string }
  | { kind: "command"; name: string; args: string[] };

// A command name must start with a letter and the whole draft must be one line. Without those
// two guards `/usr/bin/env` and pasted code blocks would be swallowed as unknown commands.
const COMMAND_PATTERN = /^\/([a-zA-Z][a-zA-Z0-9-]*)(?:\s+(.*))?$/;

export function parseCommandInput(draft: string): ParsedInput {
  const trimmed = draft.trim();
  if (trimmed.includes("\n")) return { kind: "message" };
  // `//` is the escape hatch for prose that genuinely starts with a slash; unknown commands are
  // rejected rather than forwarded, so without it such prose would be unsendable.
  if (trimmed.startsWith("//")) return { kind: "literal", content: trimmed.slice(1) };

  const match = COMMAND_PATTERN.exec(trimmed);
  if (!match) return { kind: "message" };

  const args = (match[2] ?? "").split(/\s+/).filter((part) => part.length > 0);
  return { kind: "command", name: match[1].toLowerCase(), args };
}
