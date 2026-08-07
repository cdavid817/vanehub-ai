import type { ModelFamily } from "./model-family";

export interface SeatBriefingEntry {
  /** The handle other seats type after `@` to route a turn here. */
  mention: string;
  roleName: string;
  agentName: string;
  modelFamily: ModelFamily;
  responsibility: string;
  instruction: string;
}

/**
 * Composes what a seat is told before it speaks: its own role, who else is in the room, and how
 * routing works.
 *
 * This text is the only channel through which an Agent learns the collaboration rules, so its
 * wording is behaviour, not documentation. Two things it must get right:
 *
 * - The roster. An Agent cannot hand off to a teammate it does not know exists, and it uses each
 *   teammate's responsibility to decide who is the right recipient.
 * - The line-leading rule. An Agent that does not know mentions only route at the start of a line
 *   will write "让 @代码审查 看一下" mid-sentence and nothing will happen.
 */
export function buildSeatBriefing({
  maxDepth,
  maxMentions,
  others,
  self,
}: {
  maxDepth: number;
  maxMentions: number;
  others: SeatBriefingEntry[];
  self: SeatBriefingEntry;
}): string {
  const sections = [self.instruction.trim()];

  if (others.length === 0) {
    sections.push("你是这个会话里唯一的参与者，没有可以交接的队友。");
  } else {
    const roster = others
      .map(
        (seat) =>
          `- @${seat.mention}（${seat.roleName}，由 ${seat.agentName} 承担，模型家族 ${seat.modelFamily}）：${seat.responsibility}`,
      )
      .join("\n");
    sections.push(`本次会话的其他参与者：\n${roster}`);
    sections.push(
      [
        "交接规则：",
        `- 需要某位队友接手时，把 @对方 放在**行首**单独起一行。写在句子中间不会触发交接。`,
        `- 一条回复最多 @ ${maxMentions} 位队友；连续交接最多 ${maxDepth} 轮，超出会被系统截断。`,
        "- 不要 @ 你自己，也不要在代码块里 @ 任何人。",
        "- 只有在你确实需要对方做事时才交接；仅仅提到对方的工作不必 @。",
      ].join("\n"),
    );
  }

  sections.push(
    [
      "需要人参与时，在行首 @用户，并写明意图：",
      "- `@用户 handoff` —— 你需要人做决定，工作会停下来等他。",
      "- `@用户 fyi` —— 只是让人知道一声，工作继续，不会打断他。",
      "- `@用户 done` —— 本轮工作完成。",
      "只有 handoff 会打断人，所以不要把只想告知的事写成 handoff。",
    ].join("\n"),
  );

  return sections.join("\n\n");
}
