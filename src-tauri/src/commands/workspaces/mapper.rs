use super::dto;
use crate::contexts::workspaces::api::{
    DirectoryListing, DocumentListing, FileContent, FileSearchListing, GitDiffFile, GitDiffHunk,
    GitDiffLine, GitDiffResult, GitDiffSource, GitStatusResult, KnownProject, KnownRemoteWorkspace,
    ProjectInspection, SessionLogExportResult, SessionLogQuery, SessionWorkspaceContext,
    ShellRuntimeDescriptor, WorkspaceLogLevel,
};

pub(super) fn known_project_to_dto(project: KnownProject) -> dto::KnownProject {
    dto::KnownProject {
        path: project.path,
        display_name: project.display_name,
        is_git: project.is_git,
        last_opened_at: project.last_opened_at,
    }
}

pub(super) fn known_remote_workspace_to_dto(
    workspace: KnownRemoteWorkspace,
) -> dto::KnownRemoteWorkspace {
    dto::KnownRemoteWorkspace {
        host: workspace.host,
        port: workspace.port,
        user: workspace.user,
        path: workspace.path,
        display_name: workspace.display_name,
        uri: workspace.uri,
        last_opened_at: workspace.last_opened_at,
    }
}

pub(super) fn project_inspection_to_dto(inspection: ProjectInspection) -> dto::ProjectInspection {
    dto::ProjectInspection {
        path: inspection.path().to_string(),
        display_name: inspection.display_name().to_string(),
        is_git: inspection.is_git(),
        git_root: inspection.git_root().map(str::to_string),
    }
}

fn workspace_context_to_dto(context: SessionWorkspaceContext) -> dto::SessionWorkspaceContext {
    dto::SessionWorkspaceContext {
        availability: context.availability,
        root_name: context.root_name,
        reason: context.reason,
    }
}

pub(super) fn directory_listing_to_dto(listing: DirectoryListing) -> dto::DirectoryListing {
    dto::DirectoryListing {
        context: workspace_context_to_dto(listing.context),
        path: listing.path,
        items: listing
            .items
            .into_iter()
            .map(|entry| dto::DirectoryEntry {
                name: entry.name,
                path: entry.path,
                kind: entry.kind,
                size: entry.size,
            })
            .collect(),
        truncated: listing.truncated,
        next_cursor: listing.next_cursor,
    }
}

pub(super) fn document_listing_to_dto(listing: DocumentListing) -> dto::DocumentListing {
    dto::DocumentListing {
        context: workspace_context_to_dto(listing.context),
        items: listing
            .items
            .into_iter()
            .map(|document| dto::SessionDocument {
                name: document.name,
                path: document.path,
                kind: document.kind,
            })
            .collect(),
        truncated: listing.truncated,
        next_cursor: listing.next_cursor,
    }
}

pub(super) fn file_search_listing_to_dto(listing: FileSearchListing) -> dto::FileSearchListing {
    dto::FileSearchListing {
        context: workspace_context_to_dto(listing.context),
        items: listing
            .items
            .into_iter()
            .map(|entry| dto::FileSearchMatch {
                name: entry.name,
                path: entry.path,
            })
            .collect(),
        truncated: listing.truncated,
    }
}

pub(super) fn file_content_to_dto(file: FileContent) -> dto::FileContent {
    dto::FileContent {
        path: file.path,
        name: file.name,
        status: file.status,
        size: file.size,
        content: file.content,
    }
}

pub(super) fn git_status_to_dto(status: GitStatusResult) -> dto::GitStatusResult {
    dto::GitStatusResult {
        context: workspace_context_to_dto(status.context),
        is_git: status.is_git,
        branch: status.branch,
        items: status
            .items
            .into_iter()
            .map(|entry| dto::GitStatusEntry {
                path: entry.path,
                previous_path: entry.previous_path,
                index: entry.index,
                worktree: entry.worktree,
            })
            .collect(),
        truncated: status.truncated,
        next_cursor: status.next_cursor,
    }
}

pub(super) fn git_diff_source_from_dto(source: dto::GitDiffSource) -> GitDiffSource {
    match source {
        dto::GitDiffSource::Working => GitDiffSource::Working,
        dto::GitDiffSource::Staged => GitDiffSource::Staged,
    }
}

fn git_diff_source_to_dto(source: GitDiffSource) -> dto::GitDiffSource {
    match source {
        GitDiffSource::Working => dto::GitDiffSource::Working,
        GitDiffSource::Staged => dto::GitDiffSource::Staged,
    }
}

fn git_diff_line_to_dto(line: GitDiffLine) -> dto::GitDiffLine {
    dto::GitDiffLine {
        kind: line.kind,
        content: line.content,
        old_line_number: line.old_line_number,
        new_line_number: line.new_line_number,
    }
}

fn git_diff_hunk_to_dto(hunk: GitDiffHunk) -> dto::GitDiffHunk {
    dto::GitDiffHunk {
        header: hunk.header,
        old_start: hunk.old_start,
        old_lines: hunk.old_lines,
        new_start: hunk.new_start,
        new_lines: hunk.new_lines,
        lines: hunk.lines.into_iter().map(git_diff_line_to_dto).collect(),
    }
}

fn git_diff_file_to_dto(file: GitDiffFile) -> dto::GitDiffFile {
    dto::GitDiffFile {
        old_path: file.old_path,
        new_path: file.new_path,
        binary: file.binary,
        oversized: file.oversized,
        hunks: file.hunks.into_iter().map(git_diff_hunk_to_dto).collect(),
    }
}

pub(super) fn git_diff_to_dto(diff: GitDiffResult) -> dto::GitDiffResult {
    dto::GitDiffResult {
        context: workspace_context_to_dto(diff.context),
        source: git_diff_source_to_dto(diff.source),
        files: diff.files.into_iter().map(git_diff_file_to_dto).collect(),
        truncated: diff.truncated,
    }
}

fn log_level_from_dto(level: dto::WorkspaceLogLevel) -> WorkspaceLogLevel {
    match level {
        dto::WorkspaceLogLevel::Error => WorkspaceLogLevel::Error,
        dto::WorkspaceLogLevel::Warn => WorkspaceLogLevel::Warn,
        dto::WorkspaceLogLevel::Info => WorkspaceLogLevel::Info,
        dto::WorkspaceLogLevel::Debug => WorkspaceLogLevel::Debug,
    }
}

pub(super) fn session_log_query_from_dto(query: dto::SessionLogQuery) -> SessionLogQuery {
    SessionLogQuery {
        session_id: query.session_id,
        levels: query.levels.into_iter().map(log_level_from_dto).collect(),
        search: query.search,
        seat_id: query.seat_id.filter(|value| !value.trim().is_empty()),
        cursor: query.cursor,
        limit: query.limit,
    }
}

pub(super) fn session_log_export_to_dto(
    result: SessionLogExportResult,
) -> dto::SessionLogExportResult {
    dto::SessionLogExportResult {
        status: result.status,
        path: result.path,
    }
}

pub(super) fn shell_runtime_to_dto(runtime: ShellRuntimeDescriptor) -> dto::ShellRuntimeDescriptor {
    let supports_resize = runtime.supports_resize();
    let supports_replay = runtime.supports_replay();
    let supports_reconnect = runtime.supports_reconnect();
    match runtime {
        ShellRuntimeDescriptor::Native => dto::ShellRuntimeDescriptor::Native {
            supports_resize,
            supports_replay,
            supports_reconnect,
        },
        ShellRuntimeDescriptor::Remote {
            connection_id,
            profile_revision,
            ..
        } => dto::ShellRuntimeDescriptor::Remote {
            connection_id,
            profile_revision,
            supports_resize,
            supports_replay,
            supports_reconnect,
        },
        ShellRuntimeDescriptor::Simulated => dto::ShellRuntimeDescriptor::Simulated {
            supports_resize,
            supports_replay,
            supports_reconnect,
        },
        ShellRuntimeDescriptor::Unavailable {
            reason_code,
            remediation,
        } => dto::ShellRuntimeDescriptor::Unavailable {
            reason_code,
            remediation,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_history_and_inspection_keep_the_existing_camel_case_contract() {
        let project = known_project_to_dto(KnownProject {
            path: "D:\\code\\app".to_string(),
            display_name: "app".to_string(),
            is_git: true,
            last_opened_at: "2026-07-18T12:00:00Z".to_string(),
        });
        let inspection =
            ProjectInspection::from_probe("D:\\code\\app", Some("D:\\code\\app".to_string()))
                .expect("inspection");

        assert_eq!(
            serde_json::to_value(project).expect("project DTO"),
            json!({
                "path": "D:\\code\\app",
                "displayName": "app",
                "isGit": true,
                "lastOpenedAt": "2026-07-18T12:00:00Z"
            })
        );
        assert_eq!(
            serde_json::to_value(project_inspection_to_dto(inspection)).expect("inspection DTO"),
            json!({
                "path": "D:\\code\\app",
                "displayName": "app",
                "isGit": true,
                "gitRoot": "D:\\code\\app"
            })
        );
    }

    #[test]
    fn remote_history_keeps_nullable_user_and_complete_identity_fields() {
        let remote = known_remote_workspace_to_dto(KnownRemoteWorkspace {
            host: "example.com".to_string(),
            port: 22,
            user: None,
            path: "/work/app".to_string(),
            display_name: "example.com:app".to_string(),
            uri: "ssh://example.com/work/app".to_string(),
            last_opened_at: "2026-07-18T12:00:00Z".to_string(),
        });

        assert_eq!(
            serde_json::to_value(remote).expect("remote DTO"),
            json!({
                "host": "example.com",
                "port": 22,
                "user": null,
                "path": "/work/app",
                "displayName": "example.com:app",
                "uri": "ssh://example.com/work/app",
                "lastOpenedAt": "2026-07-18T12:00:00Z"
            })
        );
    }

    #[test]
    fn workspace_query_outputs_keep_camel_case_and_lowercase_enums() {
        let context = SessionWorkspaceContext::available(Some("app".to_string()));
        let directory = directory_listing_to_dto(DirectoryListing {
            context: context.clone(),
            path: "src".to_string(),
            items: Vec::new(),
            truncated: false,
            next_cursor: None,
        });
        let file = file_content_to_dto(FileContent {
            path: "README.md".to_string(),
            name: "README.md".to_string(),
            status: "text",
            size: 7,
            content: Some("fixture".to_string()),
        });
        let diff = git_diff_to_dto(GitDiffResult {
            context,
            source: GitDiffSource::Staged,
            files: Vec::new(),
            truncated: false,
        });
        assert_eq!(
            serde_json::to_value(directory).expect("directory DTO"),
            json!({
                "context": {"availability": "available", "rootName": "app", "reason": null},
                "path": "src",
                "items": [],
                "truncated": false,
                "nextCursor": null
            })
        );
        assert_eq!(
            serde_json::to_value(file).expect("file DTO"),
            json!({
                "path": "README.md",
                "name": "README.md",
                "status": "text",
                "size": 7,
                "content": "fixture"
            })
        );
        assert_eq!(
            serde_json::to_value(diff).expect("diff DTO"),
            json!({
                "context": {"availability": "available", "rootName": "app", "reason": null},
                "source": "staged",
                "files": [],
                "truncated": false
            })
        );
    }

    #[test]
    fn workspace_query_inputs_map_transport_enums_without_leaking_serde_models() {
        let input: dto::SessionLogQuery = serde_json::from_value(json!({
            "sessionId": "session-1",
            "levels": ["error", "debug"],
            "search": "failed",
            "cursor": "20",
            "limit": 10
        }))
        .expect("log query DTO");
        let query = session_log_query_from_dto(input);
        let source: dto::GitDiffSource =
            serde_json::from_value(json!("working")).expect("diff source DTO");

        assert_eq!(query.session_id, "session-1");
        assert_eq!(
            query.levels,
            vec![WorkspaceLogLevel::Error, WorkspaceLogLevel::Debug]
        );
        assert_eq!(query.cursor.as_deref(), Some("20"));
        assert_eq!(query.limit, Some(10));
        assert_eq!(git_diff_source_from_dto(source), GitDiffSource::Working);
    }

    /// The frontend narrows on `kind`, so every variant's wire shape is pinned here rather than
    /// only the one the local development machine happens to produce.
    #[test]
    fn every_shell_runtime_descriptor_variant_keeps_its_wire_shape() {
        let remote = shell_runtime_to_dto(ShellRuntimeDescriptor::Remote {
            connection_id: "connection-7".to_string(),
            profile_revision: 3,
            supports_reconnect: false,
        });
        assert_eq!(
            serde_json::to_value(remote).expect("remote descriptor"),
            json!({
                "kind": "remote",
                "connectionId": "connection-7",
                "profileRevision": 3,
                "supportsResize": true,
                "supportsReplay": true,
                "supportsReconnect": false
            })
        );

        let simulated = shell_runtime_to_dto(ShellRuntimeDescriptor::Simulated);
        assert_eq!(
            serde_json::to_value(simulated).expect("simulated descriptor"),
            json!({
                "kind": "simulated",
                "supportsResize": false,
                "supportsReplay": true,
                "supportsReconnect": false
            })
        );

        let unavailable = shell_runtime_to_dto(ShellRuntimeDescriptor::Unavailable {
            reason_code: "workspace_provider_unavailable",
            remediation: Some("Reconnect the SSH profile.".to_string()),
        });
        assert_eq!(
            serde_json::to_value(unavailable).expect("unavailable descriptor"),
            json!({
                "kind": "unavailable",
                "reasonCode": "workspace_provider_unavailable",
                "remediation": "Reconnect the SSH profile."
            })
        );

        // Absent remediation is omitted rather than serialized as null, so the frontend's
        // optional field stays optional instead of becoming `string | null`.
        let bare = shell_runtime_to_dto(ShellRuntimeDescriptor::Unavailable {
            reason_code: "shell_not_found",
            remediation: None,
        });
        assert_eq!(
            serde_json::to_value(bare).expect("bare unavailable descriptor"),
            json!({ "kind": "unavailable", "reasonCode": "shell_not_found" })
        );
    }
}
