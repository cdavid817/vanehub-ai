#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PromptHookCategory {
    Bootstrap,
    Callback,
    Dynamic,
    Law,
    Navigation,
    Routing,
    Static,
}

impl PromptHookCategory {
    pub(crate) const ALL: [Self; 7] = [
        Self::Bootstrap,
        Self::Callback,
        Self::Dynamic,
        Self::Law,
        Self::Navigation,
        Self::Routing,
        Self::Static,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Callback => "callback",
            Self::Dynamic => "dynamic",
            Self::Law => "law",
            Self::Navigation => "navigation",
            Self::Routing => "routing",
            Self::Static => "static",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PromptHookStage {
    SessionInit,
    PerTurn,
}

impl PromptHookStage {
    pub(crate) const ALL: [Self; 2] = [Self::SessionInit, Self::PerTurn];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SessionInit => "session-init",
            Self::PerTurn => "per-turn",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PromptHookSource {
    Builtin,
    User,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_and_stage_values_are_strict_and_stable() {
        assert_eq!(
            PromptHookCategory::ALL.map(PromptHookCategory::as_str),
            [
                "bootstrap",
                "callback",
                "dynamic",
                "law",
                "navigation",
                "routing",
                "static"
            ]
        );
        assert_eq!(
            PromptHookCategory::parse("law"),
            Some(PromptHookCategory::Law)
        );
        assert_eq!(PromptHookCategory::parse("unknown"), None);
        assert_eq!(
            PromptHookStage::parse("per-turn"),
            Some(PromptHookStage::PerTurn)
        );
        assert_eq!(PromptHookStage::parse("per_turn"), None);
    }
}
