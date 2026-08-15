use super::ContextCompactionDecision;

pub(crate) const AUTOMATIC_COMPACTION_POLICY_VERSION: &str =
    "onepiece-automatic-compaction-control-v1";
pub(crate) const AUTOMATIC_COMPACTION_COOLDOWN_CHARACTERS: u64 = 8_192;
pub(crate) const AUTOMATIC_COMPACTION_FAILURE_LIMIT: u8 = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum AutomaticCompactionMode {
    #[default]
    Automatic,
    Suppressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompactionTriggerSource {
    TokenAware,
    CharacterFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthoritativeCompactionDecision {
    pub(crate) should_compact: bool,
    pub(crate) source: CompactionTriggerSource,
    pub(crate) character_decision: bool,
}

pub(crate) fn select_authoritative_compaction(
    token_decision: &ContextCompactionDecision,
    character_decision: bool,
) -> AuthoritativeCompactionDecision {
    match token_decision.should_compact {
        Some(should_compact) => AuthoritativeCompactionDecision {
            should_compact,
            source: CompactionTriggerSource::TokenAware,
            character_decision,
        },
        None => AuthoritativeCompactionDecision {
            should_compact: character_decision,
            source: CompactionTriggerSource::CharacterFallback,
            character_decision,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompactionBypassReason {
    RequestSuppressed,
    UserPreferenceSuppressed,
    Cooldown,
    CircuitOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutomaticCompactionState {
    user_preference_enabled: bool,
    last_success_characters: Option<u64>,
    consecutive_failures: u8,
    circuit_open: bool,
}

impl Default for AutomaticCompactionState {
    fn default() -> Self {
        Self::with_user_preference(true)
    }
}

impl AutomaticCompactionState {
    pub(crate) fn with_user_preference(enabled: bool) -> Self {
        Self {
            user_preference_enabled: enabled,
            last_success_characters: None,
            consecutive_failures: 0,
            circuit_open: false,
        }
    }

    pub(crate) fn bypass_reason(
        &self,
        mode: AutomaticCompactionMode,
        current_characters: u64,
    ) -> Option<CompactionBypassReason> {
        if mode == AutomaticCompactionMode::Suppressed {
            return Some(CompactionBypassReason::RequestSuppressed);
        }
        if !self.user_preference_enabled {
            return Some(CompactionBypassReason::UserPreferenceSuppressed);
        }
        if self.circuit_open {
            return Some(CompactionBypassReason::CircuitOpen);
        }
        if self.last_success_characters.is_some()
            && self.growth_since_success(current_characters)
                < AUTOMATIC_COMPACTION_COOLDOWN_CHARACTERS
        {
            return Some(CompactionBypassReason::Cooldown);
        }
        None
    }

    pub(crate) fn record_success(&mut self, post_compaction_characters: u64) {
        self.last_success_characters = Some(post_compaction_characters);
        self.consecutive_failures = 0;
        self.circuit_open = false;
    }

    pub(crate) fn record_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.circuit_open = self.consecutive_failures >= AUTOMATIC_COMPACTION_FAILURE_LIMIT;
    }

    pub(crate) fn growth_since_success(&self, current_characters: u64) -> u64 {
        self.last_success_characters
            .map_or(0, |baseline| current_characters.saturating_sub(baseline))
    }

    pub(crate) fn consecutive_failures(&self) -> u8 {
        self.consecutive_failures
    }

    pub(crate) fn circuit_open(&self) -> bool {
        self.circuit_open
    }
}
