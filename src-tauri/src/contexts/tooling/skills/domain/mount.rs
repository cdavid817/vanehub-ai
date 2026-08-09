use super::SkillDomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SkillMountPath(String);

impl SkillMountPath {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SkillDomainError> {
        let raw = value.into();
        let path = raw.trim();
        let has_drive_prefix = path.as_bytes().get(1) == Some(&b':');
        let invalid_segment = path
            .split(['/', '\\'])
            .any(|segment| segment.is_empty() || segment == "." || segment == "..");
        let first_segment = path.split(['/', '\\']).next().unwrap_or_default();
        let overlaps_managed_namespace = first_segment.eq_ignore_ascii_case(".vanehub");
        if path.is_empty()
            || path.starts_with('/')
            || path.starts_with('\\')
            || has_drive_prefix
            || invalid_segment
            || overlaps_managed_namespace
            || path.chars().any(char::is_control)
        {
            return Err(SkillDomainError::InvalidMountPath(raw));
        }
        Ok(Self(path.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Antigravity uses the agent-agnostic `.agents/skills` root rather than a tool-named one. Read
/// out of the installed `agy` binary, which carries the literal template
/// `{workspace}/.agents/skills/{skill_name}/SKILL.md` — the published documentation does not
/// state it.
pub(crate) fn default_mount_path(agent_id: &str) -> &'static str {
    match agent_id {
        "claude-code" => ".claude/skills",
        "codex-cli" => ".codex/skills",
        "gemini-cli" => ".gemini/skills",
        "opencode" => ".opencode/skills",
        "antigravity-cli" => ".agents/skills",
        _ => ".skills",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_paths_are_relative_bounded_and_preserve_known_defaults() {
        for (agent, path) in [
            ("claude-code", ".claude/skills"),
            ("codex-cli", ".codex/skills"),
            ("gemini-cli", ".gemini/skills"),
            ("opencode", ".opencode/skills"),
            ("antigravity-cli", ".agents/skills"),
            ("custom", ".skills"),
        ] {
            assert_eq!(default_mount_path(agent), path);
            assert_eq!(SkillMountPath::parse(path).expect("default").as_str(), path);
        }
    }

    #[test]
    fn mount_path_rejects_absolute_traversal_empty_and_control_character_values() {
        for path in [
            "",
            "   ",
            "../skills",
            ".codex/../skills",
            "/tmp/skills",
            "\\server\\share",
            "C:\\Users\\private",
            ".codex//skills",
            ".codex/./skills",
            ".codex/skills\nnext",
            ".vanehub",
            ".vanehub/skills",
            ".VANEHUB/skills",
        ] {
            assert!(matches!(
                SkillMountPath::parse(path),
                Err(SkillDomainError::InvalidMountPath(_))
            ));
        }
    }
}
