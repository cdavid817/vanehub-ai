// A mention token is the trailing `@...` run at the caret. Shared by the composer (which
// rewrites it on selection) and the layout model (which turns it into a search query), so
// both agree on exactly what the user is typing.
const MENTION_TOKEN = /(?:^|\s)@([^\s@]*)$/;

export function composerMentionQuery(value: string): string | null {
  return value.match(MENTION_TOKEN)?.[1]?.toLowerCase() ?? null;
}

/** Replaces the trailing mention token, preserving whichever whitespace character preceded it. */
export function replaceComposerMention(value: string, insertion: string): string {
  return value.replace(MENTION_TOKEN, (token) => {
    const leading = token.startsWith(" ") || token.startsWith("\n") ? token[0] : "";
    return `${leading}@${insertion} `;
  });
}
