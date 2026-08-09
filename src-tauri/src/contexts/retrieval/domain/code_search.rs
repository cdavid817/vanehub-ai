use super::{Degradation, MatchedVia};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeSearchQuery {
    pub(crate) text: String,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CodeSearchHit {
    pub(crate) file_path: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) language: String,
    pub(crate) symbol_name: Option<String>,
    pub(crate) symbol_kind: Option<String>,
    pub(crate) snippet: String,
    pub(crate) matched_via: MatchedVia,
    pub(crate) score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CodeSearchOutcome {
    pub(crate) hits: Vec<CodeSearchHit>,
    pub(crate) degraded: Option<Degradation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeSearchCandidate {
    pub(crate) source_id: String,
    pub(crate) file_path: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) language: String,
    pub(crate) symbol_name: Option<String>,
    pub(crate) symbol_kind: Option<String>,
    pub(crate) snippet: String,
}
