use super::ApplicationError;
use crate::contexts::operations::domain::{AgentRun, RunRunner, RunState};
use std::collections::BTreeMap;
use std::sync::Arc;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 50;

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlQuery {
    #[serde(skip)]
    pub(crate) attention_only: bool,
    #[serde(default)]
    pub(crate) cursor: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<usize>,
    #[serde(default)]
    pub(crate) states: Vec<RunState>,
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
    #[serde(default)]
    pub(crate) project_id: Option<String>,
    #[serde(default)]
    pub(crate) runner: Option<String>,
    #[serde(default)]
    pub(crate) sort: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlCounts {
    pub(crate) running: usize,
    pub(crate) waiting_approval: usize,
    pub(crate) waiting_user: usize,
    pub(crate) retrying: usize,
    pub(crate) blocked: usize,
    pub(crate) failed: usize,
    pub(crate) completed_recently: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlNavigationTarget {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) session_id: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlRunSummary {
    pub(crate) run_id: String,
    pub(crate) version: u64,
    pub(crate) owner_type: String,
    pub(crate) owner_id: String,
    pub(crate) agent_id: Option<String>,
    pub(crate) title: String,
    pub(crate) state: RunState,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) project_id: Option<String>,
    pub(crate) workspace: Option<String>,
    pub(crate) phase: Option<String>,
    pub(crate) attention: Option<String>,
    pub(crate) reason_code: Option<String>,
    pub(crate) verification: String,
    pub(crate) tokens: Option<u64>,
    pub(crate) cost: Option<f64>,
    pub(crate) actions: Vec<String>,
    pub(crate) navigation: Option<MissionControlNavigationTarget>,
    pub(crate) runner: Option<RunRunner>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlPage {
    pub(crate) items: Vec<MissionControlRunSummary>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlOverview {
    pub(crate) counts: MissionControlCounts,
    pub(crate) attention: MissionControlPage,
    pub(crate) active: MissionControlPage,
    pub(crate) recent: MissionControlPage,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlFacetAvailability {
    pub(crate) facet: String,
    pub(crate) state: String,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionControlRunDetail {
    pub(crate) run: MissionControlRunSummary,
    pub(crate) facets: Vec<MissionControlFacetAvailability>,
}

pub(crate) trait MissionControlRepository: Send + Sync {
    fn query(
        &self,
        query: &MissionControlQuery,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<AgentRun>, ApplicationError>;
    fn counts(&self) -> Result<BTreeMap<String, usize>, ApplicationError>;
    fn get(&self, run_id: &str) -> Result<AgentRun, ApplicationError>;
}

#[derive(Clone)]
pub(crate) struct MissionControlService {
    repository: Arc<dyn MissionControlRepository>,
}

impl MissionControlService {
    pub(crate) fn new(repository: Arc<dyn MissionControlRepository>) -> Self {
        Self { repository }
    }

    pub(crate) fn overview(
        &self,
        query: MissionControlQuery,
    ) -> Result<MissionControlOverview, ApplicationError> {
        if query
            .runner
            .as_deref()
            .is_some_and(|value| !matches!(value, "local" | "ssh" | "remote"))
            || query
                .sort
                .as_deref()
                .is_some_and(|value| !matches!(value, "newest" | "oldest" | "attention"))
        {
            return Err(ApplicationError::Invalid(
                "invalid mission control query".into(),
            ));
        }
        let offset = query
            .cursor
            .as_deref()
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|_| ApplicationError::Invalid("invalid mission control cursor".into()))?;
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let scoped = |scope: &[RunState],
                      attention_only: bool|
         -> Result<MissionControlPage, ApplicationError> {
            let mut section = query.clone();
            section.states = if section.states.is_empty() {
                scope.to_vec()
            } else {
                section
                    .states
                    .into_iter()
                    .filter(|state| scope.contains(state))
                    .collect()
            };
            if section.states.is_empty() {
                return Ok(MissionControlPage {
                    items: Vec::new(),
                    next_cursor: None,
                });
            }
            section.attention_only = attention_only;
            let runs = self.repository.query(&section, offset, limit + 1)?;
            let has_more = runs.len() > limit;
            Ok(MissionControlPage {
                items: runs.into_iter().take(limit).map(project).collect(),
                next_cursor: has_more.then(|| (offset + limit).to_string()),
            })
        };
        let raw = self.repository.counts()?;
        let count = |state: &str| raw.get(state).copied().unwrap_or_default();
        Ok(MissionControlOverview {
            counts: MissionControlCounts {
                running: count("running"),
                waiting_approval: count("waiting_approval"),
                waiting_user: count("waiting_user"),
                retrying: count("retrying"),
                blocked: count("blocked") + count("stuck"),
                failed: count("failed"),
                completed_recently: count("completed"),
            },
            attention: scoped(
                &[
                    RunState::Created,
                    RunState::Preparing,
                    RunState::Running,
                    RunState::WaitingApproval,
                    RunState::WaitingUser,
                    RunState::Paused,
                    RunState::Retrying,
                    RunState::Blocked,
                    RunState::Stuck,
                    RunState::Verifying,
                    RunState::Failed,
                ],
                true,
            )?,
            active: scoped(
                &[
                    RunState::Created,
                    RunState::Preparing,
                    RunState::Running,
                    RunState::WaitingApproval,
                    RunState::WaitingUser,
                    RunState::Paused,
                    RunState::Retrying,
                    RunState::Blocked,
                    RunState::Stuck,
                    RunState::Verifying,
                ],
                false,
            )?,
            recent: scoped(
                &[RunState::Completed, RunState::Failed, RunState::Cancelled],
                false,
            )?,
        })
    }

    pub(crate) fn detail(&self, run_id: &str) -> Result<MissionControlRunDetail, ApplicationError> {
        let run = self.repository.get(run_id)?;
        let links: Vec<_> = run
            .links
            .iter()
            .map(|link| link.link_type.as_str())
            .collect();
        let facets = [
            "overview",
            "timeline",
            "tools",
            "files",
            "review",
            "verification",
            "context",
            "usage",
            "logs",
        ]
        .into_iter()
        .map(|facet| MissionControlFacetAvailability {
            facet: facet.into(),
            state: if matches!(facet, "overview" | "timeline" | "logs") || links.contains(&facet) {
                "available"
            } else {
                "unavailable"
            }
            .into(),
        })
        .collect();
        Ok(MissionControlRunDetail {
            run: project(run),
            facets,
        })
    }
}

pub(crate) fn project(run: AgentRun) -> MissionControlRunSummary {
    let terminal = is_terminal(&run.state);
    let session = run.links.iter().find(|link| link.link_type == "session");
    let review = run.links.iter().find(|link| link.link_type == "review");
    let attention = match run.state {
        RunState::WaitingApproval => Some("approval"),
        RunState::WaitingUser => Some("user"),
        RunState::Blocked | RunState::Stuck => Some("stuck"),
        RunState::Failed => Some("failed"),
        _ if review.is_some() => Some("review"),
        _ => None,
    }
    .map(str::to_string);
    let mut actions = vec!["open".into()];
    if !terminal {
        actions.push("cancel".into());
    }
    if matches!(
        run.state,
        RunState::Paused | RunState::Blocked | RunState::Stuck
    ) {
        actions.push("resume".into());
    }
    if run.state == RunState::WaitingApproval {
        actions.push("approval".into());
    }
    if review.is_some() {
        actions.push("review".into());
    }
    let navigation = review
        .map(|link| MissionControlNavigationTarget {
            kind: "review".into(),
            id: link.link_id.clone(),
            session_id: session.map(|item| item.link_id.clone()),
        })
        .or_else(|| {
            session.map(|link| MissionControlNavigationTarget {
                kind: "session".into(),
                id: link.link_id.clone(),
                session_id: None,
            })
        });
    MissionControlRunSummary {
        run_id: run.id,
        version: run.version,
        owner_type: run.owner.owner_type.clone(),
        owner_id: run.owner.owner_id.clone(),
        agent_id: (run.owner.owner_type.contains("agent")
            || run.owner.owner_type.contains("generation"))
        .then(|| run.owner.owner_id.clone()),
        title: "Agent Run".into(),
        state: run.state,
        created_at: run.created_at,
        updated_at: run.updated_at.clone(),
        ended_at: terminal.then_some(run.updated_at),
        project_id: None,
        workspace: None,
        phase: Some(format!("{:?}", run.state).to_lowercase()),
        attention,
        reason_code: run.reason_code,
        verification: match run.state {
            RunState::Verifying => "running",
            RunState::Completed => "passed",
            RunState::Failed => "failed",
            _ => "unavailable",
        }
        .into(),
        tokens: None,
        cost: None,
        actions,
        navigation,
        runner: run.runner,
    }
}

fn is_terminal(state: &RunState) -> bool {
    matches!(
        state,
        RunState::Completed | RunState::Failed | RunState::Cancelled
    )
}
