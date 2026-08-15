use serde::{Deserialize, Serialize};

use crate::contexts::goals::api::{GoalDetail, GoalInput, GoalLinkTarget};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoalInputDto {
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) acceptance_notes: String,
    #[serde(default)]
    pub(crate) project_path: Option<String>,
}

impl GoalInputDto {
    /// The id is assigned by the service on create and overwritten from the
    /// route on update, so callers never supply one.
    pub(crate) fn into_domain(self) -> GoalInput {
        GoalInput {
            id: String::new(),
            title: self.title,
            description: self.description,
            acceptance_notes: self.acceptance_notes,
            project_path: self.project_path,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoalLinkDto {
    pub(crate) target_kind: String,
    pub(crate) target_id: String,
    pub(crate) progress: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GoalDto {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) acceptance_notes: String,
    /// What the goal stores. Never `awaitingAcceptance`.
    pub(crate) status: String,
    /// What the goal presents, recomputed from its children on every read.
    pub(crate) derived_status: String,
    pub(crate) project_path: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    /// Links that count toward acceptance: neither sessions nor unresolvable.
    pub(crate) counted: usize,
    pub(crate) terminal: usize,
    pub(crate) unresolvable: usize,
    pub(crate) links: Vec<GoalLinkDto>,
}

pub(crate) fn to_dto(detail: GoalDetail) -> GoalDto {
    GoalDto {
        id: detail.goal.id,
        title: detail.goal.title,
        description: detail.goal.description,
        acceptance_notes: detail.goal.acceptance_notes,
        status: detail.goal.status.as_str().to_string(),
        derived_status: detail.progress.derived_status.as_str().to_string(),
        project_path: detail.goal.project_path,
        created_at: detail.goal.created_at,
        updated_at: detail.goal.updated_at,
        counted: detail.progress.counted,
        terminal: detail.progress.terminal,
        unresolvable: detail.progress.unresolvable,
        links: detail
            .progress
            .links
            .into_iter()
            .map(|link| GoalLinkDto {
                target_kind: link.target_kind.as_str().to_string(),
                target_id: link.target_id,
                progress: link.progress.as_str().to_string(),
            })
            .collect(),
    }
}

pub(crate) fn parse_target_kind(value: &str) -> Result<GoalLinkTarget, String> {
    GoalLinkTarget::parse(value).map_err(|_| format!("Unknown goal link target \"{value}\"."))
}
