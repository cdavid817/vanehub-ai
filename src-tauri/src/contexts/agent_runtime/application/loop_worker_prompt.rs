use super::{LoopGitStateView, StartLoopWorkerRequest};

const MAX_CONTEXT_BYTES: usize = 32 * 1024;

pub(super) fn worker_prompt(
    request: &StartLoopWorkerRequest,
    git: &LoopGitStateView,
    feedback: Option<&str>,
) -> String {
    let definition = &request.definition_snapshot;
    let mut prompt = PromptBuilder::new();
    prompt.section("Goal", [definition.goal.as_str()], 4 * 1024);
    prompt.section(
        "Acceptance criteria",
        definition
            .acceptance_criteria
            .iter()
            .map(String::as_str)
            .take(12),
        5 * 1024,
    );
    prompt.section(
        "Allowed paths",
        definition.allowed_paths.iter().map(String::as_str).take(32),
        2 * 1024,
    );
    prompt.section(
        "Protected paths",
        definition
            .protected_paths
            .iter()
            .map(String::as_str)
            .take(32),
        2 * 1024,
    );
    let git_lines = std::iter::once(format!(
        "branch: {}{}",
        git.branch.as_deref().unwrap_or("unknown"),
        if git.truncated {
            " (status truncated)"
        } else {
            ""
        }
    ))
    .chain(git.entries.iter().take(100).map(|entry| {
        format!(
            "{} [index={}, worktree={}]",
            entry.path, entry.index_status, entry.worktree_status
        )
    }))
    .collect::<Vec<_>>();
    prompt.section(
        "Current Git state",
        git_lines.iter().map(String::as_str),
        6 * 1024,
    );
    let evidence_lines = request
        .prior_evidence
        .iter()
        .rev()
        .take(16)
        .map(|item| format!("{} [{}]: {}", item.kind, item.status, item.summary))
        .collect::<Vec<_>>();
    prompt.section(
        "Prior evidence (newest first)",
        evidence_lines.iter().map(String::as_str),
        6 * 1024,
    );
    prompt.section("User continuation feedback", feedback, 4 * 1024);
    let limits = &definition.limits;
    let remaining_iterations = limits.max_iterations.saturating_sub(request.sequence) + 1;
    let remaining_seconds = limits
        .total_timeout_seconds
        .saturating_sub(request.elapsed_seconds);
    let limit_lines = [
        format!("iteration: {}/{}", request.sequence, limits.max_iterations),
        format!("remaining iterations including this one: {remaining_iterations}"),
        format!("step timeout seconds: {}", limits.step_timeout_seconds),
        format!("remaining total seconds: {remaining_seconds}"),
    ];
    prompt.section(
        "Remaining limits",
        limit_lines.iter().map(String::as_str),
        1024,
    );
    prompt.finish()
}

struct PromptBuilder {
    value: String,
}

impl PromptBuilder {
    fn new() -> Self {
        Self {
            value: "Loop Worker Context\n".to_string(),
        }
    }

    fn section<'a>(
        &mut self,
        title: &str,
        lines: impl IntoIterator<Item = &'a str>,
        budget: usize,
    ) {
        self.push(&format!("\n## {title}\n"));
        let mut remaining = budget;
        let mut wrote = false;
        for line in lines {
            if remaining == 0 || self.value.len() >= MAX_CONTEXT_BYTES {
                break;
            }
            let line = truncate_utf8(line, remaining.min(1024));
            self.push("- ");
            self.push(line);
            self.push("\n");
            remaining = remaining.saturating_sub(line.len() + 3);
            wrote = true;
        }
        if !wrote {
            self.push("- none\n");
        }
    }

    fn push(&mut self, value: &str) {
        let available = MAX_CONTEXT_BYTES.saturating_sub(self.value.len());
        self.value.push_str(truncate_utf8(value, available));
    }

    fn finish(self) -> String {
        self.value
    }
}

pub(super) fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    let mut end = value.len().min(max_bytes);
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    &value[..end]
}
