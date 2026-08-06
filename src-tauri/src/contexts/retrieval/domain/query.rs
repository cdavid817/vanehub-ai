/// 检索面向的是**同一个主机级共享记忆池**——`agent-memory-shared-pool`（迁移 42）之后，
/// `agent_memories` 对每个 Agent 全量可见，最近记忆的注入也不再按 agent/folder 过滤。查询因此
/// 不带 scope：带了反而会让 `recall` 只能搜到系统提示词里已经注入过的内容的一个真子集。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetrievalQuery {
    pub(crate) text: String,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchedVia {
    Vector,
    Keyword,
    Both,
}

impl MatchedVia {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Keyword => "keyword",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Degradation {
    KeywordOnly,
    VectorOnly,
}

impl Degradation {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::KeywordOnly => "keyword_only",
            Self::VectorOnly => "vector_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoredHit {
    pub(crate) source_id: String,
    pub(crate) content: String,
    pub(crate) created_at: String,
    pub(crate) score: f64,
    pub(crate) matched_via: MatchedVia,
}

/// 把整条 query 转义成**单个 FTS5 字符串字面量**。
///
/// 仓库里唯一的既有 FTS 消费方 `contexts/workspaces/infrastructure/output_search.rs:36-47`
/// 是把原始串直接塞进 `MATCH ?1` 的，只挡空串与超长。这里不能照抄：`recall` 的 query 由模型
/// 自由生成，含 `"` `*` `:` `-` `OR` `NEAR` 时 FTS5 会按查询语法解析，轻则语义跑偏，重则整条
/// 语句报错。转义成短语后，trigram tokenizer 下的子串匹配正是我们想要的行为。
pub(crate) fn escape_fts_query(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len() + 2);
    escaped.push('"');
    for character in raw.trim().chars() {
        if character == '"' {
            escaped.push('"');
        }
        escaped.push(character);
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_query_becomes_a_quoted_phrase() {
        assert_eq!(escape_fts_query("npm not pnpm"), "\"npm not pnpm\"");
    }

    #[test]
    fn embedded_double_quotes_are_doubled_not_dropped() {
        assert_eq!(escape_fts_query("use \"npm\""), "\"use \"\"npm\"\"\"");
    }

    #[test]
    fn fts_operators_lose_their_syntactic_meaning() {
        for raw in ["a OR b", "a NEAR b", "prefix*", "col:value", "-excluded"] {
            let escaped = escape_fts_query(raw);
            assert!(
                escaped.starts_with('"') && escaped.ends_with('"'),
                "{escaped}"
            );
            assert!(
                escaped.contains(raw),
                "{escaped} should carry {raw} verbatim"
            );
        }
    }

    #[test]
    fn whitespace_only_queries_escape_to_an_empty_phrase() {
        assert_eq!(escape_fts_query("   "), "\"\"");
    }
}
