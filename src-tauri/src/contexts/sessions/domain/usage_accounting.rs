#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UsageInteractionKind {
    ManagedCli,
    TerminalCli,
    NativeApi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UsagePurpose {
    AssistantInitial,
    ToolContinuation,
    ContextCompaction,
    MemoryExtraction,
    Retry,
    TerminalInterval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UsageStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MeasurementQuality {
    Reported,
    ReportedDerived,
    Estimated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MeasurementKind {
    Interval,
    CumulativeSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AccountingUnit {
    Tokens,
    Characters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TokenOverlap {
    Subset,
    Exclusive,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TokenDimensions {
    pub(crate) input: i64,
    pub(crate) output: i64,
    pub(crate) cached_input: i64,
    pub(crate) cache_write_input: i64,
    pub(crate) reasoning_output: i64,
    pub(crate) provider_total: Option<i64>,
}

impl TokenDimensions {
    pub(crate) fn validate(self) -> Result<Self, &'static str> {
        let counts = [
            self.input,
            self.output,
            self.cached_input,
            self.cache_write_input,
            self.reasoning_output,
            self.provider_total.unwrap_or_default(),
        ];
        if counts.into_iter().any(|count| count < 0) {
            return Err("usage counts must be non-negative");
        }
        Ok(self)
    }

    pub(crate) fn is_zero(self) -> bool {
        self.input == 0
            && self.output == 0
            && self.cached_input == 0
            && self.cache_write_input == 0
            && self.reasoning_output == 0
            && self.provider_total.unwrap_or_default() == 0
    }

    pub(crate) fn headline_total(
        self,
        cache_overlap: TokenOverlap,
        reasoning_overlap: TokenOverlap,
    ) -> Option<i64> {
        if let Some(total) = self.provider_total {
            return Some(total);
        }
        if cache_overlap == TokenOverlap::Unknown || reasoning_overlap == TokenOverlap::Unknown {
            return None;
        }
        let cache = if cache_overlap == TokenOverlap::Exclusive {
            self.cached_input.checked_add(self.cache_write_input)?
        } else {
            0
        };
        let reasoning = if reasoning_overlap == TokenOverlap::Exclusive {
            self.reasoning_output
        } else {
            0
        };
        self.input
            .checked_add(self.output)?
            .checked_add(cache)?
            .checked_add(reasoning)
    }

    fn checked_delta(self, previous: Self) -> Option<Self> {
        Some(Self {
            input: self.input.checked_sub(previous.input)?,
            output: self.output.checked_sub(previous.output)?,
            cached_input: self.cached_input.checked_sub(previous.cached_input)?,
            cache_write_input: self
                .cache_write_input
                .checked_sub(previous.cache_write_input)?,
            reasoning_output: self
                .reasoning_output
                .checked_sub(previous.reasoning_output)?,
            provider_total: match (self.provider_total, previous.provider_total) {
                (Some(current), Some(previous)) => Some(current.checked_sub(previous)?),
                (None, None) => None,
                _ => return None,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CumulativeReconciliation {
    Unchanged,
    Delta(TokenDimensions),
    Reset,
}

pub(crate) fn reconcile_cumulative_usage(
    previous: TokenDimensions,
    current: TokenDimensions,
) -> CumulativeReconciliation {
    let Some(delta) = current.checked_delta(previous) else {
        return CumulativeReconciliation::Reset;
    };
    if delta.validate().is_err() {
        CumulativeReconciliation::Reset
    } else if delta.is_zero() {
        CumulativeReconciliation::Unchanged
    } else {
        CumulativeReconciliation::Delta(delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_total_wins_over_overlapping_dimensions() {
        let usage = TokenDimensions {
            input: 100,
            output: 20,
            cached_input: 80,
            reasoning_output: 10,
            provider_total: Some(120),
            ..TokenDimensions::default()
        };
        assert_eq!(
            usage.headline_total(TokenOverlap::Unknown, TokenOverlap::Unknown),
            Some(120)
        );
    }

    #[test]
    fn unknown_overlap_refuses_to_invent_a_total() {
        let usage = TokenDimensions {
            input: 100,
            output: 20,
            cached_input: 80,
            ..TokenDimensions::default()
        };
        assert_eq!(
            usage.headline_total(TokenOverlap::Unknown, TokenOverlap::Subset),
            None
        );
    }

    #[test]
    fn cumulative_reconciliation_detects_delta_unchanged_and_reset() {
        let first = TokenDimensions {
            input: 10,
            output: 5,
            provider_total: Some(15),
            ..TokenDimensions::default()
        };
        let second = TokenDimensions {
            input: 14,
            output: 6,
            provider_total: Some(20),
            ..TokenDimensions::default()
        };
        assert_eq!(
            reconcile_cumulative_usage(first, second),
            CumulativeReconciliation::Delta(TokenDimensions {
                input: 4,
                output: 1,
                provider_total: Some(5),
                ..TokenDimensions::default()
            })
        );
        assert_eq!(
            reconcile_cumulative_usage(second, second),
            CumulativeReconciliation::Unchanged
        );
        assert_eq!(
            reconcile_cumulative_usage(second, first),
            CumulativeReconciliation::Reset
        );
    }

    #[test]
    fn negative_and_overflowing_counts_are_rejected() {
        assert!(TokenDimensions {
            input: -1,
            ..TokenDimensions::default()
        }
        .validate()
        .is_err());
        assert_eq!(
            TokenDimensions {
                input: i64::MAX,
                output: 1,
                ..TokenDimensions::default()
            }
            .headline_total(TokenOverlap::Subset, TokenOverlap::Subset),
            None
        );
    }
}
