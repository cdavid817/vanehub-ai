use super::{SkillId, SkillLocation, SkillScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillDriftIssueType {
    MissingSource,
    MetadataChanged,
    UnregisteredSource,
    MissingMount,
    Conflict,
    DeletedBuiltin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillMountObservation {
    Managed,
    Missing,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillBindingInspection {
    pub(crate) agent_id: String,
    pub(crate) mounted_path: String,
    pub(crate) observation: SkillMountObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillSourceInspection {
    Missing { path: String },
    Present { path: String, content_hash: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredSkillInspection {
    pub(crate) id: SkillId,
    pub(crate) enabled: bool,
    pub(crate) expected_content_hash: String,
    pub(crate) source: SkillSourceInspection,
    pub(crate) bindings: Vec<SkillBindingInspection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnregisteredSkillInspection {
    pub(crate) id: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDriftInspection {
    pub(crate) location: SkillLocation,
    pub(crate) registered: Vec<RegisteredSkillInspection>,
    pub(crate) unregistered_sources: Vec<UnregisteredSkillInspection>,
    pub(crate) deleted_builtin_ids: Vec<SkillId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDriftIssue {
    pub(crate) skill_id: String,
    pub(crate) issue_type: SkillDriftIssueType,
    pub(crate) agent_id: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) message: &'static str,
}

pub(crate) fn detect_drift(inspection: &SkillDriftInspection) -> Vec<SkillDriftIssue> {
    let mut issues = Vec::new();
    for skill in &inspection.registered {
        match &skill.source {
            SkillSourceInspection::Missing { path } => {
                issues.push(issue(
                    skill.id.as_str(),
                    SkillDriftIssueType::MissingSource,
                    None,
                    Some(path.clone()),
                    "SKILL.md is missing",
                ));
                continue;
            }
            SkillSourceInspection::Present { path, content_hash }
                if content_hash != &skill.expected_content_hash =>
            {
                issues.push(issue(
                    skill.id.as_str(),
                    SkillDriftIssueType::MetadataChanged,
                    None,
                    Some(path.clone()),
                    "SKILL.md differs from the registry snapshot",
                ));
            }
            SkillSourceInspection::Present { .. } => {}
        }

        if skill.enabled {
            for binding in &skill.bindings {
                let (issue_type, message) = match binding.observation {
                    SkillMountObservation::Managed => continue,
                    SkillMountObservation::Missing => {
                        (SkillDriftIssueType::MissingMount, "Agent mount is missing")
                    }
                    SkillMountObservation::Conflict => (
                        SkillDriftIssueType::Conflict,
                        "Agent mount path is occupied by unmanaged content",
                    ),
                };
                issues.push(issue(
                    skill.id.as_str(),
                    issue_type,
                    Some(binding.agent_id.clone()),
                    Some(binding.mounted_path.clone()),
                    message,
                ));
            }
        }
    }

    for source in &inspection.unregistered_sources {
        issues.push(issue(
            &source.id,
            SkillDriftIssueType::UnregisteredSource,
            None,
            Some(source.path.clone()),
            "Skill source exists without a registry record",
        ));
    }

    if inspection.location.scope == SkillScope::Global {
        for id in &inspection.deleted_builtin_ids {
            issues.push(issue(
                id.as_str(),
                SkillDriftIssueType::DeletedBuiltin,
                None,
                None,
                "Built-in Skill is deleted and can be restored",
            ));
        }
    }
    issues
}

fn issue(
    skill_id: &str,
    issue_type: SkillDriftIssueType,
    agent_id: Option<String>,
    path: Option<String>,
    message: &'static str,
) -> SkillDriftIssue {
    SkillDriftIssue {
        skill_id: skill_id.to_string(),
        issue_type,
        agent_id,
        path,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> SkillId {
        SkillId::parse(value).expect("id")
    }

    fn location(scope: SkillScope) -> SkillLocation {
        SkillLocation::new(
            scope,
            (scope == SkillScope::Workspace).then_some("D:/workspace"),
        )
        .expect("location")
    }

    #[test]
    fn drift_rules_classify_source_hash_mount_conflict_and_unregistered_observations() {
        let issues = detect_drift(&SkillDriftInspection {
            location: location(SkillScope::Workspace),
            registered: vec![
                RegisteredSkillInspection {
                    id: id("missing-source"),
                    enabled: true,
                    expected_content_hash: "expected".to_string(),
                    source: SkillSourceInspection::Missing {
                        path: "D:/workspace/.vanehub/skills/missing-source/SKILL.md".to_string(),
                    },
                    bindings: vec![SkillBindingInspection {
                        agent_id: "codex-cli".to_string(),
                        mounted_path: "ignored-after-missing-source".to_string(),
                        observation: SkillMountObservation::Missing,
                    }],
                },
                RegisteredSkillInspection {
                    id: id("changed-skill"),
                    enabled: true,
                    expected_content_hash: "expected".to_string(),
                    source: SkillSourceInspection::Present {
                        path: "changed/SKILL.md".to_string(),
                        content_hash: "changed".to_string(),
                    },
                    bindings: vec![
                        SkillBindingInspection {
                            agent_id: "codex-cli".to_string(),
                            mounted_path: ".codex/skills/changed-skill".to_string(),
                            observation: SkillMountObservation::Missing,
                        },
                        SkillBindingInspection {
                            agent_id: "claude-code".to_string(),
                            mounted_path: ".claude/skills/changed-skill".to_string(),
                            observation: SkillMountObservation::Conflict,
                        },
                    ],
                },
            ],
            unregistered_sources: vec![UnregisteredSkillInspection {
                id: "external-skill".to_string(),
                path: "external-skill".to_string(),
            }],
            deleted_builtin_ids: vec![id("code-review")],
        });

        assert_eq!(
            issues
                .iter()
                .map(|issue| issue.issue_type)
                .collect::<Vec<_>>(),
            vec![
                SkillDriftIssueType::MissingSource,
                SkillDriftIssueType::MetadataChanged,
                SkillDriftIssueType::MissingMount,
                SkillDriftIssueType::Conflict,
                SkillDriftIssueType::UnregisteredSource,
            ]
        );
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.skill_id == "missing-source")
                .count(),
            1
        );
    }

    #[test]
    fn disabled_skills_skip_mount_drift_and_deleted_builtins_are_global_only() {
        let registered = vec![RegisteredSkillInspection {
            id: id("disabled-skill"),
            enabled: false,
            expected_content_hash: "same".to_string(),
            source: SkillSourceInspection::Present {
                path: "SKILL.md".to_string(),
                content_hash: "same".to_string(),
            },
            bindings: vec![SkillBindingInspection {
                agent_id: "codex-cli".to_string(),
                mounted_path: ".codex/skills/disabled-skill".to_string(),
                observation: SkillMountObservation::Missing,
            }],
        }];
        let workspace = detect_drift(&SkillDriftInspection {
            location: location(SkillScope::Workspace),
            registered: registered.clone(),
            unregistered_sources: Vec::new(),
            deleted_builtin_ids: vec![id("code-review")],
        });
        assert!(workspace.is_empty());

        let global = detect_drift(&SkillDriftInspection {
            location: location(SkillScope::Global),
            registered,
            unregistered_sources: Vec::new(),
            deleted_builtin_ids: vec![id("code-review")],
        });
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].issue_type, SkillDriftIssueType::DeletedBuiltin);
    }
}
