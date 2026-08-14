use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItemSourceLink {
    pub(crate) source_kind: String,
    pub(crate) source_id: String,
    pub(crate) relation: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) available: bool,
    pub(crate) project_path: Option<String>,
    pub(crate) updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItem {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) stage: String,
    pub(crate) priority: String,
    pub(crate) rank: i64,
    pub(crate) project_path: Option<String>,
    pub(crate) due_at: Option<String>,
    pub(crate) archived: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) sources: Vec<WorkItemSourceLink>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkItemFilters {
    #[serde(default)]
    pub(crate) archived: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWorkItemInput {
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) stage: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateWorkItemInput {
    pub(crate) work_item_id: String,
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) project_path: Option<Option<String>>,
    pub(crate) due_at: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoveWorkItemInput {
    pub(crate) work_item_id: String,
    pub(crate) stage: String,
    pub(crate) before_work_item_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinkWorkItemSourceInput {
    pub(crate) work_item_id: String,
    pub(crate) source_kind: String,
    pub(crate) source_id: String,
    pub(crate) relation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanSummary {
    pub(crate) id: String,
    pub(crate) goal: String,
    pub(crate) project_path: String,
    pub(crate) status: String,
    pub(crate) latest_run_id: Option<String>,
    pub(crate) latest_run_status: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}
