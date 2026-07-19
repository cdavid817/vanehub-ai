use crate::contexts::operations::api::{OperationKind, OperationsApi};
use crate::contexts::sessions::application::{
    SessionCreationOperation, SessionOperationPort, SessionRecord, SessionsApplicationError,
};
use serde::Serialize;

#[derive(Clone)]
pub(crate) struct SessionOperationAdapter {
    operations: OperationsApi,
}

impl SessionOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl SessionOperationPort for SessionOperationAdapter {
    fn start_session_creation(
        &self,
        related_entity_id: Option<String>,
    ) -> Result<SessionCreationOperation, SessionsApplicationError> {
        self.operations
            .start(
                OperationKind::Workspace,
                related_entity_id,
                Some("Creating session".to_string()),
            )
            .map(|operation| SessionCreationOperation {
                id: operation.id,
                related_entity_id: operation.related_entity_id,
                message: operation.message,
                created_at: operation.created_at,
                updated_at: operation.updated_at,
            })
            .map_err(operation_error)
    }

    fn append_log(&self, operation_id: &str, line: String) -> Result<(), SessionsApplicationError> {
        self.operations
            .append_log(operation_id, line)
            .map(|_| ())
            .map_err(operation_error)
    }

    fn complete_session_creation(
        &self,
        operation_id: &str,
        session: &SessionRecord,
    ) -> Result<(), SessionsApplicationError> {
        let payload = serde_json::to_value(SessionOperationPayload::from(session))
            .map_err(|error| SessionsApplicationError::Serialization(error.to_string()))?;
        self.operations
            .complete(operation_id, Some(payload))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn fail_session_creation(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<(), SessionsApplicationError> {
        self.operations
            .fail(operation_id, error)
            .map(|_| ())
            .map_err(operation_error)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionOperationPayload<'a> {
    id: &'a str,
    title: &'a str,
    agent_id: &'a str,
    interaction_mode: &'a str,
    lifecycle_state: &'a str,
    folder: &'a Option<String>,
    project_path: &'a Option<String>,
    worktree_path: &'a Option<String>,
    worktree_name: &'a Option<String>,
    worktree_branch: &'a Option<String>,
    remote_workspace: Option<RemoteWorkspacePayload<'a>>,
    runtime_session_id: &'a Option<String>,
    category_id: Option<&'a str>,
    source: SessionSourcePayload<'a>,
    pinned: bool,
    archived: bool,
    created_at: &'a str,
    updated_at: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteWorkspacePayload<'a> {
    host: &'a str,
    user: &'a Option<String>,
    path: &'a str,
    display_name: &'a str,
    uri: &'a str,
}

#[derive(Serialize)]
struct SessionSourcePayload<'a> {
    kind: &'a str,
    connector: Option<&'a str>,
}

impl<'a> From<&'a SessionRecord> for SessionOperationPayload<'a> {
    fn from(session: &'a SessionRecord) -> Self {
        Self {
            id: session.id(),
            title: session.aggregate.title().as_str(),
            agent_id: &session.agent_id,
            interaction_mode: &session.interaction_mode,
            lifecycle_state: session.aggregate.lifecycle().as_str(),
            folder: &session.workspace.folder,
            project_path: &session.workspace.project_path,
            worktree_path: &session.workspace.worktree_path,
            worktree_name: &session.workspace.worktree_name,
            worktree_branch: &session.workspace.worktree_branch,
            remote_workspace: session
                .workspace
                .remote_workspace
                .as_ref()
                .map(|workspace| RemoteWorkspacePayload {
                    host: &workspace.host,
                    user: &workspace.user,
                    path: &workspace.path,
                    display_name: &workspace.display_name,
                    uri: &workspace.uri,
                }),
            runtime_session_id: &session.runtime_session_id,
            category_id: session.aggregate.category_id().map(|id| id.as_str()),
            source: SessionSourcePayload {
                kind: session.aggregate.owner().kind(),
                connector: session
                    .aggregate
                    .owner()
                    .connector_id()
                    .and_then(transport_connector),
            },
            pinned: session.aggregate.is_pinned(),
            archived: session.aggregate.is_archived(),
            created_at: &session.created_at,
            updated_at: &session.updated_at,
        }
    }
}

fn transport_connector(connector: &str) -> Option<&'static str> {
    match connector {
        "feishu" => Some("feishu"),
        "telegram" => Some("telegram"),
        "dingtalk" => Some("ding-talk"),
        "wecom" => Some("we-com"),
        "weixin" | "wechat" => Some("weixin"),
        _ => None,
    }
}

fn operation_error(error: impl std::fmt::Display) -> SessionsApplicationError {
    SessionsApplicationError::Operation(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::sessions::application::{SessionRemoteWorkspace, SessionWorkspace};
    use crate::contexts::sessions::domain::{
        SessionAggregate, SessionId, SessionLifecycle, SessionOwner, SessionTitle,
    };

    #[test]
    fn operation_payload_keeps_the_existing_session_transport_shape() {
        let session = SessionRecord {
            aggregate: SessionAggregate::rehydrate(
                SessionId::parse("session-operation").expect("id"),
                SessionTitle::for_creation(Some("Operation")),
                SessionLifecycle::Idle,
                SessionOwner::connector("dingtalk").expect("owner"),
                None,
                false,
                false,
            ),
            agent_id: "codex-cli".to_string(),
            interaction_mode: "cli".to_string(),
            workspace: SessionWorkspace {
                folder: Some("ssh://dev@example.com/work/app".to_string()),
                remote_workspace: Some(SessionRemoteWorkspace {
                    host: "example.com".to_string(),
                    user: Some("dev".to_string()),
                    path: "/work/app".to_string(),
                    display_name: "App".to_string(),
                    uri: "ssh://dev@example.com/work/app".to_string(),
                }),
                ..Default::default()
            },
            runtime_session_id: None,
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        };

        let value =
            serde_json::to_value(SessionOperationPayload::from(&session)).expect("serialize");

        assert_eq!(value["interactionMode"], "cli");
        assert_eq!(value["lifecycleState"], "idle");
        assert_eq!(value["source"]["kind"], "im");
        assert_eq!(value["source"]["connector"], "ding-talk");
        assert_eq!(value["remoteWorkspace"]["user"], "dev");
        assert!(value.get("interaction_mode").is_none());
    }
}
