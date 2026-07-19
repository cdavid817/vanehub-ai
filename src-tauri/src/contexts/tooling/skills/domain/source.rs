#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SkillSource {
    Builtin,
    User,
    Imported,
}

impl SkillSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User => "user",
            Self::Imported => "imported",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "builtin" => Some(Self::Builtin),
            "user" => Some(Self::User),
            "imported" => Some(Self::Imported),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_values_are_stable_and_unknown_values_do_not_fall_back() {
        for (value, source) in [
            ("builtin", SkillSource::Builtin),
            ("user", SkillSource::User),
            ("imported", SkillSource::Imported),
        ] {
            assert_eq!(SkillSource::parse(value), Some(source));
            assert_eq!(source.as_str(), value);
        }
        assert_eq!(SkillSource::parse("external"), None);
    }
}
