//! Routing decisions for a multi-seat session's turn loop.
//!
//! These live in the native layer rather than the frontend because a session can run with no UI
//! attached — IM connectors and scheduled tasks both start sessions headlessly. Routing from the
//! frontend would mean such a session never hands off at all.
//!
//! Mirrors `src/services/mention-routing.ts`, `turn-routing.ts`, and `human-handoff.ts`.
//!
//! Nothing calls these yet: the turn coordinator that drives them is task 7.2, and the decisions
//! were ported first so the coordinator has something tested to stand on. The `allow` below goes
//! away with that task — see
//! `docs/superpowers/plans/2026-08-07-multi-agent-turn-coordinator-handoff.md`.
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChainEndReason {
    TooManyMentions,
    MaxDepth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NextTurn {
    pub(crate) targets: Vec<String>,
    /// `None` when the chain simply ran out of mentions, which is a finished round rather than a
    /// failure. Conflating the two would make every normal ending look broken.
    pub(crate) ended_reason: Option<ChainEndReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HumanHandoffIntent {
    Handoff,
    Fyi,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HumanHandoffEffect {
    pub(crate) turn_holder_is_human: bool,
    pub(crate) round_complete: bool,
    pub(crate) starts_waiting: bool,
}

const USER_MENTION: &str = "@用户";

/// Quote and list markers still count as the start of a line: an Agent writing a checklist is
/// still addressing someone.
fn strip_line_prefix(line: &str) -> &str {
    let mut rest = line.trim_start();
    loop {
        let next = if let Some(stripped) = rest.strip_prefix('>') {
            stripped.trim_start()
        } else if let Some(stripped) = rest
            .strip_prefix("- ")
            .or_else(|| rest.strip_prefix("* "))
            .or_else(|| rest.strip_prefix("+ "))
        {
            stripped.trim_start()
        } else if let Some(stripped) = strip_ordered_marker(rest) {
            stripped.trim_start()
        } else {
            return rest;
        };
        if next == rest {
            return rest;
        }
        rest = next;
    }
}

fn strip_ordered_marker(value: &str) -> Option<&str> {
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .count();
    if digits == 0 {
        return None;
    }
    let rest = &value[digits..];
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    rest.strip_prefix(' ')
}

/// A handle ends at whitespace or punctuation. Without this, `@opus-45` would match `@opus`.
fn is_boundary(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            ',' | '.'
                | ':'
                | ';'
                | '!'
                | '?'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | '，'
                | '。'
                | '！'
                | '？'
                | '、'
                | '：'
                | '；'
                | '（'
                | '）'
                | '【'
                | '】'
                | '《'
                | '》'
                | '「'
                | '」'
                | '『'
                | '』'
                | '〈'
                | '〉'
        )
}

/// An Agent explaining how routing works will paste an example; that must not dispatch anyone.
fn strip_fenced_code(text: &str) -> String {
    let mut kept = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            kept.push(line);
        }
    }
    kept.join("\n")
}

/// Finds the seats a completed reply hands off to.
///
/// Only a mention at the start of a line routes. Matching anywhere makes ordinary prose
/// unpredictable — describing a teammate's work must not dispatch them.
pub(crate) fn parse_handoff_mentions(
    text: &str,
    mentions: &[String],
    self_mention: Option<&str>,
    max_mentions: usize,
) -> NextTurn {
    // Longest first, so a handle that prefixes another cannot shadow it.
    let mut ordered: Vec<&String> = mentions.iter().collect();
    ordered.sort_by_key(|mention| std::cmp::Reverse(mention.len()));

    let mut found: Vec<String> = Vec::new();
    let mut truncated = false;
    let stripped = strip_fenced_code(text);

    for line in stripped.lines() {
        let rest = strip_line_prefix(line);
        let Some(candidate) = rest.strip_prefix('@') else {
            continue;
        };
        let matched = ordered.iter().find(|mention| {
            candidate.starts_with(mention.as_str())
                && candidate[mention.len()..]
                    .chars()
                    .next()
                    .is_none_or(is_boundary)
        });
        let Some(handle) = matched else {
            continue;
        };
        let handle = (*handle).clone();
        if self_mention == Some(handle.as_str()) || found.contains(&handle) {
            continue;
        }
        if found.len() >= max_mentions {
            truncated = true;
            continue;
        }
        found.push(handle);
    }

    NextTurn {
        targets: found,
        ended_reason: truncated.then_some(ChainEndReason::TooManyMentions),
    }
}

/// Which seats a completed reply hands off to, respecting the chain depth limit.
///
/// The depth limit exists because agents mention each other autonomously; without it a pair can
/// ping-pong indefinitely. When it fires the reason is surfaced rather than the chain silently
/// stopping, so a user is not left wondering why nobody answered.
pub(crate) fn next_turn_targets(
    reply: &str,
    mentions: &[String],
    speaker: &str,
    depth: usize,
    max_depth: usize,
    max_mentions: usize,
) -> NextTurn {
    if depth >= max_depth {
        return NextTurn {
            targets: Vec::new(),
            ended_reason: Some(ChainEndReason::MaxDepth),
        };
    }
    parse_handoff_mentions(reply, mentions, Some(speaker), max_mentions)
}

/// Reads how an Agent handed back to the human.
///
/// A bare `@用户` with no intent is informational, not blocking. Defaulting to blocking would
/// punish an Agent for mentioning the human at all, and it would learn to stop — exactly the
/// visibility loss the three intents exist to prevent.
pub(crate) fn parse_human_handoff(reply: &str) -> Option<HumanHandoffIntent> {
    let stripped = strip_fenced_code(reply);
    for line in stripped.lines() {
        let rest = strip_line_prefix(line);
        let Some(remainder) = rest.strip_prefix(USER_MENTION) else {
            continue;
        };
        let remainder = remainder.trim().to_ascii_lowercase();
        if remainder.starts_with("handoff") {
            return Some(HumanHandoffIntent::Handoff);
        }
        if remainder.starts_with("done") {
            return Some(HumanHandoffIntent::Done);
        }
        return Some(HumanHandoffIntent::Fyi);
    }
    None
}

/// Only `handoff` interrupts. That separation is the point: a single blocking "notify the human"
/// action teaches Agents to avoid notifying.
pub(crate) fn apply_human_handoff(intent: HumanHandoffIntent) -> HumanHandoffEffect {
    match intent {
        HumanHandoffIntent::Fyi => HumanHandoffEffect {
            turn_holder_is_human: false,
            round_complete: false,
            starts_waiting: false,
        },
        HumanHandoffIntent::Handoff => HumanHandoffEffect {
            turn_holder_is_human: true,
            round_complete: false,
            starts_waiting: true,
        },
        HumanHandoffIntent::Done => HumanHandoffEffect {
            turn_holder_is_human: true,
            round_complete: true,
            starts_waiting: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mentions() -> Vec<String> {
        ["架构师", "代码审查", "实现者", "opus", "opus-45"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn parse(text: &str) -> NextTurn {
        parse_handoff_mentions(text, &mentions(), Some("架构师"), 2)
    }

    #[test]
    fn routes_a_mention_at_the_start_of_a_line() {
        assert_eq!(parse("做完了。\n@代码审查 帮我看下").targets, ["代码审查"]);
    }

    #[test]
    fn ignores_a_mention_in_the_middle_of_a_line() {
        assert!(parse("做完了，让 @代码审查 看一下").targets.is_empty());
    }

    #[test]
    fn allows_whitespace_quote_and_list_prefixes() {
        for line in [
            "  @代码审查 看下",
            "- @代码审查 看下",
            "> @代码审查 看下",
            "1. @代码审查 看下",
        ] {
            assert_eq!(parse(line).targets, ["代码审查"], "failed for {line}");
        }
    }

    #[test]
    fn ignores_mentions_inside_fenced_code() {
        let text = "示例：\n```\n@代码审查 这样写\n```\n结束";
        assert!(parse(text).targets.is_empty());
    }

    #[test]
    fn filters_a_self_mention() {
        assert!(parse("@架构师 继续").targets.is_empty());
    }

    #[test]
    fn prefers_the_longest_matching_handle() {
        assert_eq!(parse("@opus-45 上").targets, ["opus-45"]);
    }

    #[test]
    fn requires_a_token_boundary() {
        assert!(parse("@代码审查者 看下").targets.is_empty());
    }

    #[test]
    fn caps_targets_and_reports_truncation() {
        let result = parse("@实现者 a\n@代码审查 b\n@opus c");
        assert_eq!(result.targets, ["实现者", "代码审查"]);
        assert_eq!(result.ended_reason, Some(ChainEndReason::TooManyMentions));
    }

    #[test]
    fn does_not_repeat_a_seat() {
        assert_eq!(parse("@代码审查 a\n@代码审查 b").targets, ["代码审查"]);
    }

    #[test]
    fn stops_at_the_chain_depth_and_says_why() {
        let result = next_turn_targets("@代码审查 继续", &mentions(), "实现者", 15, 15, 2);
        assert!(result.targets.is_empty());
        assert_eq!(result.ended_reason, Some(ChainEndReason::MaxDepth));
    }

    #[test]
    fn a_reply_naming_nobody_ends_the_chain_quietly() {
        let result = next_turn_targets("做完了。", &mentions(), "实现者", 1, 15, 2);
        assert!(result.targets.is_empty());
        assert_eq!(result.ended_reason, None);
    }

    #[test]
    fn reads_the_three_human_intents() {
        assert_eq!(
            parse_human_handoff("@用户 handoff 定一下"),
            Some(HumanHandoffIntent::Handoff)
        );
        assert_eq!(
            parse_human_handoff("@用户 fyi 顺带一提"),
            Some(HumanHandoffIntent::Fyi)
        );
        assert_eq!(
            parse_human_handoff("@用户 done 完成"),
            Some(HumanHandoffIntent::Done)
        );
    }

    #[test]
    fn a_bare_user_mention_is_informational() {
        assert_eq!(
            parse_human_handoff("@用户 我改完了"),
            Some(HumanHandoffIntent::Fyi)
        );
    }

    #[test]
    fn ignores_a_mid_line_user_mention() {
        assert_eq!(parse_human_handoff("这个要问 @用户 handoff 一下"), None);
    }

    #[test]
    fn only_a_blocking_handoff_interrupts() {
        assert!(!apply_human_handoff(HumanHandoffIntent::Fyi).turn_holder_is_human);
        assert!(apply_human_handoff(HumanHandoffIntent::Handoff).starts_waiting);
        assert!(apply_human_handoff(HumanHandoffIntent::Done).round_complete);
        assert!(!apply_human_handoff(HumanHandoffIntent::Done).starts_waiting);
    }
}
