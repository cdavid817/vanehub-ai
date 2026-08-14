use std::collections::HashMap;

use super::*;

struct Source {
    values: HashMap<
        ContextReinjectionKind,
        Result<Vec<AuthoritativeContextValue>, ContextReinjectionFailure>,
    >,
}

impl AuthoritativeContextPort for Source {
    fn load_current(
        &self,
        kind: ContextReinjectionKind,
    ) -> Result<Vec<AuthoritativeContextValue>, ContextReinjectionFailure> {
        self.values
            .get(&kind)
            .cloned()
            .unwrap_or(Err(ContextReinjectionFailure::SourceUnavailable))
    }
}

fn value(kind: ContextReinjectionKind, revision: &str, content: &str) -> AuthoritativeContextValue {
    AuthoritativeContextValue {
        kind,
        revision: revision.to_string(),
        content: content.to_string(),
    }
}

#[test]
fn resolves_current_memory_and_runtime_context_with_safe_evidence() {
    let source = Source {
        values: HashMap::from([
            (
                ContextReinjectionKind::Memory,
                Ok(vec![value(
                    ContextReinjectionKind::Memory,
                    "memory-r2",
                    "current memory",
                )]),
            ),
            (
                ContextReinjectionKind::RuntimeContext,
                Ok(vec![value(
                    ContextReinjectionKind::RuntimeContext,
                    "runtime-r4",
                    "current runtime context",
                )]),
            ),
        ]),
    };
    let result = ContextReinjectionService::resolve(
        &source,
        &[
            ContextReinjectionKind::Memory,
            ContextReinjectionKind::RuntimeContext,
        ],
        ContextReinjectionBudget::default(),
    );
    let ContextReinjectionResult::Ready(values) = result else {
        panic!("expected ready reinjection");
    };
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].content, "current memory");
    assert_eq!(values[0].evidence.revision, "memory-r2");
    let diagnostics = format!(
        "{:?}",
        values
            .iter()
            .map(|value| &value.evidence)
            .collect::<Vec<_>>()
    );
    assert!(!diagnostics.contains("current memory"));
    assert!(!diagnostics.contains("current runtime context"));
}

#[test]
fn unavailable_or_invalid_sources_preserve_history() {
    let unavailable = Source {
        values: HashMap::new(),
    };
    assert_eq!(
        ContextReinjectionService::resolve(
            &unavailable,
            &[ContextReinjectionKind::Memory],
            ContextReinjectionBudget::default(),
        ),
        ContextReinjectionResult::PreserveHistory(ContextReinjectionFailure::SourceUnavailable)
    );

    for invalid in [
        value(
            ContextReinjectionKind::Memory,
            "revision with raw text",
            "memory",
        ),
        value(ContextReinjectionKind::RuntimeContext, "r1", "memory"),
        value(ContextReinjectionKind::Memory, "r1", "   "),
    ] {
        let source = Source {
            values: HashMap::from([(ContextReinjectionKind::Memory, Ok(vec![invalid]))]),
        };
        assert!(matches!(
            ContextReinjectionService::resolve(
                &source,
                &[ContextReinjectionKind::Memory],
                ContextReinjectionBudget::default(),
            ),
            ContextReinjectionResult::PreserveHistory(_)
        ));
    }
}

#[test]
fn item_kind_and_aggregate_budget_overflow_preserve_history_without_partial_values() {
    let cases = [
        (
            vec![value(ContextReinjectionKind::Memory, "r1", "12345")],
            ContextReinjectionBudget {
                per_item_characters: 4,
                per_kind_characters: 10,
                aggregate_characters: 10,
            },
            ContextReinjectionFailure::ItemBudgetExceeded,
        ),
        (
            vec![
                value(ContextReinjectionKind::Memory, "r1", "123"),
                value(ContextReinjectionKind::Memory, "r2", "456"),
            ],
            ContextReinjectionBudget {
                per_item_characters: 5,
                per_kind_characters: 5,
                aggregate_characters: 10,
            },
            ContextReinjectionFailure::KindBudgetExceeded,
        ),
        (
            vec![value(ContextReinjectionKind::Memory, "r1", "123456")],
            ContextReinjectionBudget {
                per_item_characters: 10,
                per_kind_characters: 10,
                aggregate_characters: 5,
            },
            ContextReinjectionFailure::AggregateBudgetExceeded,
        ),
    ];
    for (values, budget, expected) in cases {
        let source = Source {
            values: HashMap::from([(ContextReinjectionKind::Memory, Ok(values))]),
        };
        assert_eq!(
            ContextReinjectionService::resolve(&source, &[ContextReinjectionKind::Memory], budget,),
            ContextReinjectionResult::PreserveHistory(expected)
        );
    }
}
