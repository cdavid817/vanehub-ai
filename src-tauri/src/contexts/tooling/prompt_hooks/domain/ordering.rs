use super::{PromptHookCategory, PromptHookDomainError, PromptHookStage};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PromptHookOrder(i64);

impl PromptHookOrder {
    pub(crate) fn new(value: i64) -> Result<Self, PromptHookDomainError> {
        if value < 0 {
            Err(PromptHookDomainError::NegativeOrder)
        } else {
            Ok(Self(value))
        }
    }

    pub(crate) fn value(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PromptHookOrderSlot {
    pub(crate) stage: PromptHookStage,
    pub(crate) category: PromptHookCategory,
    pub(crate) order: PromptHookOrder,
}

impl PromptHookOrderSlot {
    pub(crate) fn new(
        stage: PromptHookStage,
        category: PromptHookCategory,
        order: PromptHookOrder,
    ) -> Self {
        Self {
            stage,
            category,
            order,
        }
    }
}

pub(crate) fn ensure_order_available(
    requested: PromptHookOrderSlot,
    occupied: &[PromptHookOrderSlot],
) -> Result<(), PromptHookDomainError> {
    if occupied.contains(&requested) {
        Err(PromptHookDomainError::DuplicateOrder)
    } else {
        Ok(())
    }
}

pub(crate) fn compare_prompt_hook_order(
    left: (PromptHookStage, PromptHookCategory, i64, &str),
    right: (PromptHookStage, PromptHookCategory, i64, &str),
) -> Ordering {
    left.0
        .as_str()
        .cmp(right.0.as_str())
        .then(left.1.as_str().cmp(right.1.as_str()))
        .then(left.2.cmp(&right.2))
        .then(left.3.cmp(right.3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_is_non_negative() {
        assert_eq!(PromptHookOrder::new(0).expect("order").value(), 0);
        assert_eq!(
            PromptHookOrder::new(-1),
            Err(PromptHookDomainError::NegativeOrder)
        );
    }

    #[test]
    fn effective_order_preserves_stage_category_order_and_identity_tie_breaking() {
        let mut hooks = [
            (
                PromptHookStage::SessionInit,
                PromptHookCategory::Law,
                200,
                "z",
            ),
            (
                PromptHookStage::PerTurn,
                PromptHookCategory::Routing,
                600,
                "r",
            ),
            (
                PromptHookStage::PerTurn,
                PromptHookCategory::Dynamic,
                500,
                "b",
            ),
            (
                PromptHookStage::PerTurn,
                PromptHookCategory::Dynamic,
                500,
                "a",
            ),
        ];
        hooks.sort_by(|left, right| compare_prompt_hook_order(*left, *right));
        assert_eq!(hooks.map(|hook| hook.3), ["a", "b", "r", "z"]);
    }

    #[test]
    fn order_slots_are_unique_only_within_the_same_stage_and_category() {
        let requested = PromptHookOrderSlot::new(
            PromptHookStage::PerTurn,
            PromptHookCategory::Dynamic,
            PromptHookOrder::new(450).expect("order"),
        );
        assert_eq!(
            ensure_order_available(requested, &[requested]),
            Err(PromptHookDomainError::DuplicateOrder)
        );
        assert_eq!(
            ensure_order_available(
                requested,
                &[PromptHookOrderSlot::new(
                    PromptHookStage::SessionInit,
                    PromptHookCategory::Dynamic,
                    PromptHookOrder::new(450).expect("order"),
                )]
            ),
            Ok(())
        );
    }
}
