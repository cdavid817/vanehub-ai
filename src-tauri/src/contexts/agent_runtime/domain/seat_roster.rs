//! What a seat is told before it speaks, and how seats are named to each other.
//!
//! Composed natively rather than on the frontend because a session can run with no UI attached —
//! IM connectors and scheduled tasks both start sessions headlessly, and a briefing built in the
//! renderer would never reach them.

/// A normalized model family. An Agent's `provider` holds free-form display text such as
/// `"OpenAI"`, so cross-family checks must go through this rather than comparing those strings.
///
/// Mirrors `src/services/model-family.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelFamily {
    Anthropic,
    OpenAi,
    Google,
    Unknown,
}

impl ModelFamily {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
            Self::Google => "google",
            Self::Unknown => "unknown",
        }
    }
}

/// One participant as the others read it when deciding whom to hand off to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatBriefingEntry {
    /// The handle other seats type after `@` to route a turn here.
    pub(crate) mention: String,
    pub(crate) role_name: String,
    pub(crate) agent_name: String,
    pub(crate) model_family: ModelFamily,
    pub(crate) responsibility: String,
    pub(crate) instruction: String,
}

/// One attributed turn of the shared thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatTurn {
    pub(crate) speaker: String,
    pub(crate) content: String,
}

/// How a seat receives what happened before its turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SeatContextMode {
    /// The Agent's own session already holds the history and nothing is injected.
    Resume,
    Inject,
}

/// The prior conversation handed to a seat, and how it got there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeatContext {
    pub(crate) mode: SeatContextMode,
    pub(crate) text: String,
}

/// Turns role names into handles that address exactly one seat.
///
/// A session may hold two seats with the same role — two reviewers is a reasonable line-up — so a
/// repeated name is suffixed rather than rejected. Whitespace is collapsed because a handle is
/// typed after `@`, where whitespace ends the token.
pub(crate) fn derive_mentions(role_names: &[String]) -> Vec<String> {
    let mut used: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut mentions = Vec::with_capacity(role_names.len());
    for (index, name) in role_names.iter().enumerate() {
        let base = name.split_whitespace().collect::<Vec<_>>().join("-");
        let base = if base.is_empty() {
            format!("席位{}", index + 1)
        } else {
            base
        };
        let seen = used.entry(base.clone()).or_insert(0);
        *seen += 1;
        mentions.push(if *seen == 1 {
            base
        } else {
            format!("{base}-{seen}")
        });
    }
    mentions
}

/// Built-in agents are keyed by stable id, which cannot drift the way display text can.
fn family_by_agent_id(agent_id: &str) -> Option<ModelFamily> {
    match agent_id {
        "claude-code" => Some(ModelFamily::Anthropic),
        "codex-cli" => Some(ModelFamily::OpenAi),
        "gemini-cli" => Some(ModelFamily::Google),
        // Antigravity speaks Google's own CodeAssist surface and serves Google models, so its
        // family is fixed the way Gemini's is rather than user-configurable like OpenCode's.
        "antigravity-cli" => Some(ModelFamily::Google),
        // OpenCode drives whichever model the user configured, so it has no fixed family. Claiming
        // one would make a cross-family reviewer check act on a false premise.
        "opencode" => Some(ModelFamily::Unknown),
        _ => None,
    }
}

/// Resolves an Agent to a comparable model family, by stable id first and display text second.
pub(crate) fn normalize_model_family(
    agent_id: &str,
    provider: &str,
    endpoint_type: Option<&str>,
) -> ModelFamily {
    if let Some(family) = family_by_agent_id(agent_id) {
        return family;
    }
    let normalized: String = provider
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '_' && *character != '-')
        .flat_map(char::to_lowercase)
        .collect();
    let by_provider = match normalized.as_str() {
        "anthropic" | "claude" => Some(ModelFamily::Anthropic),
        "openai" | "azureopenai" => Some(ModelFamily::OpenAi),
        "google" | "gemini" | "googleai" => Some(ModelFamily::Google),
        _ => None,
    };
    if let Some(family) = by_provider {
        return family;
    }
    match endpoint_type {
        Some("anthropic-messages") => ModelFamily::Anthropic,
        Some("openai-chat-completions" | "openai-responses") => ModelFamily::OpenAi,
        _ => ModelFamily::Unknown,
    }
}

/// Composes what a seat is told before it speaks: its own role, who else is in the room, and how
/// routing works.
///
/// This text is the only channel through which an Agent learns the collaboration rules, so its
/// wording is behaviour, not documentation. Two things it must get right:
///
/// - The roster. An Agent cannot hand off to a teammate it does not know exists, and it uses each
///   teammate's responsibility to decide who is the right recipient.
/// - The line-leading rule. An Agent that does not know mentions only route at the start of a line
///   will write "让 @代码审查 看一下" mid-sentence and nothing will happen.
pub(crate) fn build_seat_briefing(
    seat: &SeatBriefingEntry,
    others: &[SeatBriefingEntry],
    max_depth: usize,
    max_mentions: usize,
) -> String {
    let mut sections = vec![seat.instruction.trim().to_string()];

    if others.is_empty() {
        sections.push("你是这个会话里唯一的参与者，没有可以交接的队友。".to_string());
    } else {
        let roster = others
            .iter()
            .map(|other| {
                format!(
                    "- @{}（{}，由 {} 承担，模型家族 {}）：{}",
                    other.mention,
                    other.role_name,
                    other.agent_name,
                    other.model_family.as_str(),
                    other.responsibility
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!("本次会话的其他参与者：\n{roster}"));
        sections.push(
            [
                "交接规则：".to_string(),
                "- 需要某位队友接手时，把 @对方 放在**行首**单独起一行。写在句子中间不会触发交接。"
                    .to_string(),
                format!(
                    "- 一条回复最多 @ {max_mentions} 位队友；连续交接最多 {max_depth} 轮，超出会被系统截断。"
                ),
                "- 不要 @ 你自己，也不要在代码块里 @ 任何人。".to_string(),
                "- 只有在你确实需要对方做事时才交接；仅仅提到对方的工作不必 @。".to_string(),
            ]
            .join("\n"),
        );
    }

    sections.push(
        [
            "需要人参与时，在行首 @用户，并写明意图：",
            "- `@用户 handoff` —— 你需要人做决定，工作会停下来等他。",
            "- `@用户 fyi` —— 只是让人知道一声，工作继续，不会打断他。",
            "- `@用户 done` —— 本轮工作完成。",
            "只有 handoff 会打断人，所以不要把只想告知的事写成 handoff。",
        ]
        .join("\n"),
    );

    sections.join("\n\n")
}

/// Decides how a seat learns what happened before its turn.
///
/// Resume first: when the seat's Agent has a provider session, its history is already there and
/// re-injecting it would pay for the same context twice. Otherwise the preceding turns are injected
/// as attributed text — this is also how a seat added mid-session catches up on work it never saw.
///
/// When the budget is tight the *most recent* turns are kept: the newest exchange is what the seat
/// is being asked to act on, while the oldest is the most likely to be recoverable from the project
/// itself.
pub(crate) fn build_seat_context(
    turns: &[SeatTurn],
    provider_session_id: Option<&str>,
    max_chars: usize,
) -> SeatContext {
    if provider_session_id.is_some_and(|value| !value.is_empty()) {
        return SeatContext {
            mode: SeatContextMode::Resume,
            text: String::new(),
        };
    }

    let mut lines: Vec<String> = Vec::new();
    let mut used = 0usize;
    for turn in turns.iter().rev() {
        let line = format!("[{} 说] {}", turn.speaker, turn.content);
        // Budgets count characters rather than bytes: these threads are mostly Chinese, and a byte
        // budget would silently admit a third as much text.
        let length = line.chars().count();
        let cost = if lines.is_empty() { length } else { length + 1 };
        if used + cost > max_chars {
            break;
        }
        lines.insert(0, line);
        used += cost;
    }
    SeatContext {
        mode: SeatContextMode::Inject,
        text: lines.join("\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(mention: &str, role: &str, agent: &str) -> SeatBriefingEntry {
        SeatBriefingEntry {
            mention: mention.to_string(),
            role_name: role.to_string(),
            agent_name: agent.to_string(),
            model_family: ModelFamily::Anthropic,
            responsibility: format!("{role}的职责"),
            instruction: format!("你是{role}。"),
        }
    }

    #[test]
    fn a_mention_is_the_role_name() {
        assert_eq!(
            derive_mentions(&["架构师".to_string(), "代码审查".to_string()]),
            ["架构师", "代码审查"]
        );
    }

    /// Two reviewers are allowed, so their handles must still address exactly one seat.
    #[test]
    fn repeated_role_names_get_distinct_handles() {
        assert_eq!(
            derive_mentions(&[
                "代码审查".to_string(),
                "代码审查".to_string(),
                "代码审查".to_string()
            ]),
            ["代码审查", "代码审查-2", "代码审查-3"]
        );
    }

    /// A handle is typed after `@`, where whitespace ends the token.
    #[test]
    fn whitespace_in_a_role_name_is_collapsed() {
        assert_eq!(
            derive_mentions(&["Code Reviewer".to_string()]),
            ["Code-Reviewer"]
        );
    }

    #[test]
    fn a_nameless_seat_still_gets_a_handle() {
        assert_eq!(
            derive_mentions(&["".to_string(), "  ".to_string()]),
            ["席位1", "席位2"]
        );
    }

    #[test]
    fn built_in_agents_normalize_by_id_not_by_display_text() {
        assert_eq!(
            normalize_model_family("claude-code", "whatever", None),
            ModelFamily::Anthropic
        );
        assert_eq!(
            normalize_model_family("codex-cli", "", None),
            ModelFamily::OpenAi
        );
        assert_eq!(
            normalize_model_family("gemini-cli", "", None),
            ModelFamily::Google
        );
    }

    /// OpenCode drives whichever model the user configured, so claiming a family would make a
    /// cross-family reviewer check act on a false premise.
    #[test]
    fn opencode_has_no_fixed_family() {
        assert_eq!(
            normalize_model_family("opencode", "Anthropic", None),
            ModelFamily::Unknown
        );
    }

    #[test]
    fn free_form_provider_text_normalizes() {
        for provider in ["OpenAI", "azure openai", "Azure_OpenAI"] {
            assert_eq!(
                normalize_model_family("custom-1", provider, None),
                ModelFamily::OpenAi,
                "failed for {provider}"
            );
        }
        assert_eq!(
            normalize_model_family("custom-2", "Claude", None),
            ModelFamily::Anthropic
        );
    }

    #[test]
    fn an_endpoint_type_settles_an_unrecognized_provider() {
        assert_eq!(
            normalize_model_family("custom-3", "Acme Cloud", Some("anthropic-messages")),
            ModelFamily::Anthropic
        );
        assert_eq!(
            normalize_model_family("custom-4", "Acme Cloud", None),
            ModelFamily::Unknown
        );
    }

    #[test]
    fn a_briefing_names_the_teammates_and_the_line_leading_rule() {
        let briefing = build_seat_briefing(
            &entry("架构师", "架构师", "Claude Code"),
            &[entry("代码审查", "代码审查", "Codex CLI")],
            15,
            2,
        );
        assert!(briefing.starts_with("你是架构师。"));
        assert!(briefing.contains("@代码审查"));
        assert!(briefing.contains("代码审查的职责"));
        assert!(briefing.contains("行首"));
        assert!(briefing.contains("最多 @ 2 位"));
        assert!(briefing.contains("最多 15 轮"));
    }

    /// An Agent told to hand off with nobody to hand off to would invent a teammate.
    #[test]
    fn a_lone_seat_is_told_it_has_no_teammates() {
        let briefing = build_seat_briefing(&entry("架构师", "架构师", "Claude Code"), &[], 15, 2);
        assert!(briefing.contains("唯一的参与者"));
        assert!(!briefing.contains("@代码审查"));
    }

    /// Three intents exist so that informing the human stays cheap enough to keep doing.
    #[test]
    fn every_briefing_explains_the_three_human_intents() {
        let briefing = build_seat_briefing(&entry("架构师", "架构师", "Claude Code"), &[], 15, 2);
        for intent in ["@用户 handoff", "@用户 fyi", "@用户 done"] {
            assert!(briefing.contains(intent), "missing {intent}");
        }
    }

    #[test]
    fn a_seat_with_a_provider_session_resumes_instead_of_being_re_injected() {
        let context = build_seat_context(
            &[SeatTurn {
                speaker: "架构师".to_string(),
                content: "做完了".to_string(),
            }],
            Some("provider-session-1"),
            1000,
        );
        assert_eq!(context.mode, SeatContextMode::Resume);
        assert!(context.text.is_empty());
    }

    #[test]
    fn prior_turns_are_injected_with_their_speaker() {
        let context = build_seat_context(
            &[
                SeatTurn {
                    speaker: "用户".to_string(),
                    content: "改下登录".to_string(),
                },
                SeatTurn {
                    speaker: "架构师".to_string(),
                    content: "方案如下".to_string(),
                },
            ],
            None,
            1000,
        );
        assert_eq!(context.mode, SeatContextMode::Inject);
        assert_eq!(context.text, "[用户 说] 改下登录\n[架构师 说] 方案如下");
    }

    /// The newest exchange is what the seat is being asked to act on; the oldest is the most
    /// likely to be recoverable from the project itself.
    #[test]
    fn a_tight_budget_keeps_the_most_recent_turns() {
        let context = build_seat_context(
            &[
                SeatTurn {
                    speaker: "用户".to_string(),
                    content: "第一条".to_string(),
                },
                SeatTurn {
                    speaker: "架构师".to_string(),
                    content: "第二条".to_string(),
                },
            ],
            None,
            "[架构师 说] 第二条".chars().count(),
        );
        assert_eq!(context.text, "[架构师 说] 第二条");
    }

    #[test]
    fn a_budget_too_small_for_any_turn_injects_nothing() {
        let context = build_seat_context(
            &[SeatTurn {
                speaker: "架构师".to_string(),
                content: "第一条".to_string(),
            }],
            None,
            3,
        );
        assert_eq!(context.mode, SeatContextMode::Inject);
        assert!(context.text.is_empty());
    }
}
