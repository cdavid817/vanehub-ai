use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopGitStateEntryView, LoopGitStatePort, LoopGitStateView,
    LoopProjectPort, LoopVerifierContextPort, PreparedLoopWorktree,
};
use crate::contexts::workspaces::api::{
    ensure_git_worktree_available, GitDiffResult, GitDiffSource, WorkspaceApi,
};

const VERIFIER_DIFF_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub(crate) struct WorkspaceLoopProjectAdapter {
    workspaces: WorkspaceApi,
}

impl WorkspaceLoopProjectAdapter {
    pub(crate) fn new(workspaces: WorkspaceApi) -> Self {
        Self { workspaces }
    }
}

impl LoopProjectPort for WorkspaceLoopProjectAdapter {
    fn validate_local_git_project(
        &self,
        project_path: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let inspection = self
            .workspaces
            .inspect_project(project_path)
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))?;
        ensure_git_worktree_available(inspection.is_git())
            .map_err(|error| AgentRuntimeApplicationError::Validation(error.to_string()))?;
        inspection.git_root().map(str::to_string).ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "Loop project must be a local Git repository.".to_string(),
            )
        })
    }

    fn prepare_loop_worktree(
        &self,
        project_path: &str,
        name: &str,
        base_branch: &str,
    ) -> Result<PreparedLoopWorktree, AgentRuntimeApplicationError> {
        self.workspaces
            .create_guarded_loop_worktree(project_path, name, base_branch)
            .map(|created| PreparedLoopWorktree {
                path: created.path,
                name: created.name,
                branch: created.branch,
            })
            .map_err(loop_error)
    }
}

impl LoopGitStatePort for WorkspaceLoopProjectAdapter {
    fn snapshot(&self, session_id: &str) -> Result<LoopGitStateView, AgentRuntimeApplicationError> {
        self.workspaces
            .get_session_git_status(session_id)
            .map(|status| LoopGitStateView {
                branch: status.branch,
                entries: status
                    .items
                    .into_iter()
                    .map(|entry| LoopGitStateEntryView {
                        path: entry.path,
                        index_status: entry.index,
                        worktree_status: entry.worktree,
                    })
                    .collect(),
                truncated: status.truncated,
            })
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))
    }
}

impl LoopVerifierContextPort for WorkspaceLoopProjectAdapter {
    fn bounded_diff(&self, session_id: &str) -> Result<String, AgentRuntimeApplicationError> {
        let status = self
            .workspaces
            .get_session_git_status(session_id)
            .map_err(loop_error)?;
        let mut output = String::new();
        for item in status.items {
            for source in [GitDiffSource::Working, GitDiffSource::Staged] {
                let diff = self
                    .workspaces
                    .get_session_git_diff(session_id, &item.path, source)
                    .map_err(loop_error)?;
                append_diff(&mut output, &diff);
                if output.len() >= VERIFIER_DIFF_BYTES {
                    const MARKER: &str = "\n[diff truncated]";
                    truncate_utf8(&mut output, VERIFIER_DIFF_BYTES - MARKER.len());
                    output.push_str(MARKER);
                    return Ok(output);
                }
            }
        }
        if output.is_empty() {
            output.push_str("No working tree diff.");
        }
        Ok(output)
    }
}

fn append_diff(output: &mut String, diff: &GitDiffResult) {
    for file in &diff.files {
        output.push_str(&format!("\n--- {:?}: {}\n", diff.source, file.new_path));
        if file.binary || file.oversized {
            output.push_str("[binary or oversized diff]\n");
            continue;
        }
        for hunk in &file.hunks {
            output.push_str(&hunk.header);
            output.push('\n');
            for line in &hunk.lines {
                let prefix = match line.kind.as_str() {
                    "addition" => '+',
                    "deletion" => '-',
                    _ => ' ',
                };
                output.push(prefix);
                output.push_str(&line.content);
                output.push('\n');
            }
        }
    }
}

fn truncate_utf8(value: &mut String, max_bytes: usize) {
    let mut boundary = max_bytes.min(value.len());
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
}

fn loop_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(error.to_string())
}
