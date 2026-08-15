#![allow(dead_code)]

use sha2::{Digest, Sha256};

use super::AuthoritativeContextPort;

const MAX_REINJECTION_ITEMS: usize = 32;
const MAX_REVISION_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ContextReinjectionKind {
    Memory,
    RuntimeContext,
}

impl ContextReinjectionKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::RuntimeContext => "runtime-context",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthoritativeContextValue {
    pub(crate) kind: ContextReinjectionKind,
    pub(crate) revision: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextReinjectionBudget {
    pub(crate) per_item_characters: u64,
    pub(crate) per_kind_characters: u64,
    pub(crate) aggregate_characters: u64,
}

impl Default for ContextReinjectionBudget {
    fn default() -> Self {
        Self {
            per_item_characters: 4_000,
            per_kind_characters: 8_000,
            aggregate_characters: 12_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextReinjectionEvidence {
    pub(crate) kind: &'static str,
    pub(crate) revision: String,
    pub(crate) source_fingerprint: String,
    pub(crate) characters: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReinjectedContextValue {
    pub(crate) kind: ContextReinjectionKind,
    pub(crate) content: String,
    pub(crate) evidence: ContextReinjectionEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextReinjectionFailure {
    SourceUnavailable,
    InvalidRevision,
    InvalidSourceKind,
    EmptySource,
    TooManyItems,
    ItemBudgetExceeded,
    KindBudgetExceeded,
    AggregateBudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContextReinjectionResult {
    Ready(Vec<ReinjectedContextValue>),
    PreserveHistory(ContextReinjectionFailure),
}

pub(crate) struct ContextReinjectionService;

impl ContextReinjectionService {
    pub(crate) fn resolve(
        port: &dyn AuthoritativeContextPort,
        kinds: &[ContextReinjectionKind],
        budget: ContextReinjectionBudget,
    ) -> ContextReinjectionResult {
        let mut output = Vec::new();
        let mut aggregate = 0_u64;
        for kind in kinds.iter().copied() {
            let values = match port.load_current(kind) {
                Ok(values) if !values.is_empty() => values,
                Ok(_) => {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::SourceUnavailable,
                    )
                }
                Err(reason) => return ContextReinjectionResult::PreserveHistory(reason),
            };
            if values.len() > MAX_REINJECTION_ITEMS {
                return ContextReinjectionResult::PreserveHistory(
                    ContextReinjectionFailure::TooManyItems,
                );
            }
            let mut kind_total = 0_u64;
            for value in values {
                if value.kind != kind {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::InvalidSourceKind,
                    );
                }
                if !safe_revision(&value.revision) {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::InvalidRevision,
                    );
                }
                let content = value.content.trim();
                if content.is_empty() {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::EmptySource,
                    );
                }
                let characters = content.chars().count() as u64;
                if characters > budget.per_item_characters {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::ItemBudgetExceeded,
                    );
                }
                kind_total = kind_total.saturating_add(characters);
                if kind_total > budget.per_kind_characters {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::KindBudgetExceeded,
                    );
                }
                aggregate = aggregate.saturating_add(characters);
                if aggregate > budget.aggregate_characters {
                    return ContextReinjectionResult::PreserveHistory(
                        ContextReinjectionFailure::AggregateBudgetExceeded,
                    );
                }
                output.push(ReinjectedContextValue {
                    kind,
                    content: content.to_owned(),
                    evidence: ContextReinjectionEvidence {
                        kind: kind.as_str(),
                        revision: value.revision,
                        source_fingerprint: fingerprint(content),
                        characters,
                    },
                });
            }
        }
        ContextReinjectionResult::Ready(output)
    }
}

fn safe_revision(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REVISION_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
