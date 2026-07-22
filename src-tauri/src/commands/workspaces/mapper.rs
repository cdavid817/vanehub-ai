use super::dto;
use crate::contexts::workspaces::api::{
    CreateShellRequest, DirectoryListing, DocumentListing, FileContent, GitDiffFile, GitDiffHunk,
    GitDiffLine, GitDiffResult, GitDiffSource, GitStatusResult, KnownProject, KnownRemoteWorkspace,
    ProjectInspection, ResizeShellRequest, SessionLogExportResult, SessionLogPage, SessionLogQuery,
    SessionWorkspaceContext, ShellSession, WorkspaceLogLevel,
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

fn log_level_to_dto(level: WorkspaceLogLevel) -> dto::WorkspaceLogLevel {
    match level {
        WorkspaceLogLevel::Error => dto::WorkspaceLogLevel::Error,
        WorkspaceLogLevel::Warn => dto::WorkspaceLogLevel::Warn,
        WorkspaceLogLevel::Info => dto::WorkspaceLogLevel::Info,
        WorkspaceLogLevel::Debug => dto::WorkspaceLogLevel::Debug,
    }
}

pub(super) fn session_log_query_from_dto(query: dto::SessionLogQuery) -> SessionLogQuery {
    SessionLogQuery {
        session_id: query.session_id,
        levels: query.levels.into_iter().map(log_level_from_dto).collect(),
        search: query.search,
        cursor: query.cursor,
        limit: query.limit,
    }
}

pub(super) fn session_log_page_to_dto(page: SessionLogPage) -> dto::SessionLogPage {
    dto::SessionLogPage {
        items: page
            .items
            .into_iter()
            .map(|entry| dto::SessionLogEntry {
                id: entry.id,
                timestamp: entry.timestamp,
                level: log_level_to_dto(entry.level),
                category: entry.category,
                message: entry.message,
                context: entry.context,
            })
            .collect(),
        truncated: page.truncated,
        next_cursor: page.next_cursor,
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

pub(super) fn create_shell_request_from_dto(input: dto::CreateShellInput) -> CreateShellRequest {
    CreateShellRequest {
        session_id: input.session_id,
        rows: input.rows,
        cols: input.cols,
    }
}

pub(super) fn resize_shell_request_from_dto(input: dto::ResizeShellInput) -> ResizeShellRequest {
    ResizeShellRequest {
        shell_id: input.shell_id,
        rows: input.rows,
        cols: input.cols,
    }
}

pub(super) fn shell_session_to_dto(shell: ShellSession) -> dto::ShellSession {
    dto::ShellSession {
        shell_id: shell.shell_id,
        session_id: shell.session_id,
        state: shell.state,
        capability: shell.capability,
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
        let logs = session_log_page_to_dto(SessionLogPage {
            items: Vec::new(),
            truncated: true,
            next_cursor: Some("25".to_string()),
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
        assert_eq!(
            serde_json::to_value(logs).expect("log DTO"),
            json!({"items": [], "truncated": true, "nextCursor": "25"})
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

    #[test]
    fn shell_command_dtos_preserve_camel_case_input_and_session_output() {
        let create: dto::CreateShellInput = serde_json::from_value(json!({
            "sessionId": "session-1",
            "rows": 24,
            "cols": 80
        }))
        .expect("create shell DTO");
        let resize: dto::ResizeShellInput = serde_json::from_value(json!({
            "shellId": "shell-1",
            "rows": 30,
            "cols": 100
        }))
        .expect("resize shell DTO");
        let create = create_shell_request_from_dto(create);
        let resize = resize_shell_request_from_dto(resize);
        let output = shell_session_to_dto(ShellSession {
            shell_id: "shell-1".to_string(),
            session_id: "session-1".to_string(),
            state: "connected",
            capability: "native",
        });

        assert_eq!(create.session_id, "session-1");
        assert_eq!((create.rows, create.cols), (24, 80));
        assert_eq!(resize.shell_id, "shell-1");
        assert_eq!((resize.rows, resize.cols), (30, 100));
        assert_eq!(
            serde_json::to_value(output).expect("shell session DTO"),
            json!({
                "shellId": "shell-1",
                "sessionId": "session-1",
                "state": "connected",
                "capability": "native"
            })
        );
    }
}
