//! The wire shapes for the retained Session Shell commands, and the one place they are built.
//!
//! Separate from `dto.rs` because these describe a Shell that outlives its view, and the older
//! shapes describe one that does not. Mixing them would invite a mapper to reach for whichever
//! `ShellSession` it found first.

use super::dto::ShellRuntimeDescriptor;
use super::mapper::shell_runtime_to_dto;
use crate::contexts::workspaces::api::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellDescriptor, SessionShellError, ShellAttachSnapshot, ShellAttachmentScope, ShellId,
    WriteSessionShellRequest,
};
use crate::contexts::workspaces::domain::{
    ShellAttachmentId, ShellCreateRequestId, ShellOutputFrame, ShellReplayGap, ShellTitle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionShell {
    pub(crate) shell_id: String,
    pub(crate) session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) seat_id: Option<String>,
    pub(crate) title: String,
    pub(crate) runtime: ShellRuntimeDescriptor,
    pub(crate) state: &'static str,
    /// Present only for the states that carry one, so a reader never has to decide whether an empty
    /// reason means "no reason" or "reason not reported".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i32>,
    pub(crate) created_at: String,
    pub(crate) last_activity_at: String,
    pub(crate) revision: u64,
    /// `unknown` is a value, not a missing field. A view that received nothing here would have to
    /// invent an answer, and the one it would invent is "nothing is running".
    pub(crate) foreground_process: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellOutputFrameDto {
    pub(crate) shell_id: String,
    pub(crate) sequence: u64,
    pub(crate) occurred_at: String,
    pub(crate) stream: &'static str,
    pub(crate) data: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellReplayGapDto {
    pub(crate) from_sequence: u64,
    pub(crate) to_sequence: u64,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellAttachment {
    /// This view's claim on the Shell. Every later write, resize, and detach carries it back, which
    /// is what lets the registry refuse input from a view that has since been replaced.
    pub(crate) attachment_id: String,
    pub(crate) descriptor: SessionShell,
    pub(crate) replay: Vec<ShellOutputFrameDto>,
    pub(crate) next_sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gap: Option<ShellReplayGapDto>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateSessionShellInput {
    pub(crate) session_id: String,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
    #[serde(default)]
    pub(crate) seat_id: Option<String>,
    /// The client's idempotency key. Absent means "the default Shell for this session and seat".
    #[serde(default)]
    pub(crate) request_id: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    /// Where the Shell starts, relative to the workspace root. Absent means the root.
    #[serde(default)]
    pub(crate) working_directory: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachSessionShellInput {
    pub(crate) shell_id: String,
    /// The last sequence the view consumed. Absent is 0, which asks for everything retained.
    #[serde(default)]
    pub(crate) after_sequence: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellAttachmentInput {
    pub(crate) shell_id: String,
    pub(crate) attachment_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WriteSessionShellInput {
    pub(crate) shell_id: String,
    pub(crate) attachment_id: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResizeSessionShellInput {
    pub(crate) shell_id: String,
    pub(crate) attachment_id: String,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RenameSessionShellInput {
    pub(crate) shell_id: String,
    pub(crate) title: String,
}

/// Parsed at the boundary rather than deeper in, so an id that cannot be a key never becomes one.
pub(super) fn shell_id(value: &str) -> Result<ShellId, SessionShellError> {
    ShellId::parse(value)
}

pub(super) fn attachment_scope(
    input: &ShellAttachmentInput,
) -> Result<ShellAttachmentScope, SessionShellError> {
    Ok(ShellAttachmentScope {
        shell_id: ShellId::parse(input.shell_id.as_str())?,
        attachment_id: ShellAttachmentId::parse(input.attachment_id.as_str())?,
    })
}

pub(super) fn create_request(
    input: CreateSessionShellInput,
) -> Result<CreateSessionShellRequest, SessionShellError> {
    let request_id = blank_to_none(input.request_id)
        .map(ShellCreateRequestId::parse)
        .transpose()?;
    let title = blank_to_none(input.title)
        .map(ShellTitle::parse)
        .transpose()?;
    Ok(CreateSessionShellRequest {
        session_id: input.session_id,
        seat_id: blank_to_none(input.seat_id),
        rows: input.rows,
        cols: input.cols,
        request_id,
        title,
        working_directory: blank_to_none(input.working_directory),
    })
}

pub(super) fn attach_request(
    input: AttachSessionShellInput,
) -> Result<AttachSessionShellRequest, SessionShellError> {
    Ok(AttachSessionShellRequest {
        shell_id: ShellId::parse(input.shell_id)?,
        after_sequence: input.after_sequence.unwrap_or_default(),
    })
}

pub(super) fn write_request(
    input: WriteSessionShellInput,
) -> Result<WriteSessionShellRequest, SessionShellError> {
    Ok(WriteSessionShellRequest {
        scope: ShellAttachmentScope {
            shell_id: ShellId::parse(input.shell_id)?,
            attachment_id: ShellAttachmentId::parse(input.attachment_id)?,
        },
        content: input.content,
    })
}

pub(super) fn resize_request(
    input: ResizeSessionShellInput,
) -> Result<ResizeSessionShellRequest, SessionShellError> {
    Ok(ResizeSessionShellRequest {
        scope: ShellAttachmentScope {
            shell_id: ShellId::parse(input.shell_id)?,
            attachment_id: ShellAttachmentId::parse(input.attachment_id)?,
        },
        rows: input.rows,
        cols: input.cols,
    })
}

/// An empty string is a client that had nothing to send, not a value. Treating it as one would make
/// an untouched rename field into a title the registry has to reject.
fn blank_to_none(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

pub(super) fn descriptor_to_dto(descriptor: SessionShellDescriptor) -> SessionShell {
    SessionShell {
        shell_id: descriptor.shell_id.as_str().to_string(),
        session_id: descriptor.session_id,
        seat_id: descriptor.seat_id,
        title: descriptor.title.as_str().to_string(),
        runtime: shell_runtime_to_dto(descriptor.runtime),
        state: descriptor.state.token(),
        reason: descriptor.state.reason().map(str::to_string),
        exit_code: descriptor.state.exit_code(),
        created_at: descriptor.created_at,
        last_activity_at: descriptor.last_activity_at,
        revision: descriptor.revision,
        foreground_process: descriptor.foreground_process.token(),
    }
}

pub(super) fn attachment_to_dto(snapshot: ShellAttachSnapshot) -> ShellAttachment {
    let shell_id = snapshot.descriptor.shell_id.as_str().to_string();
    ShellAttachment {
        attachment_id: snapshot.attachment_id.as_str().to_string(),
        replay: snapshot
            .replay
            .into_iter()
            .map(|frame| frame_to_dto(&shell_id, frame))
            .collect(),
        next_sequence: snapshot.next_sequence,
        gap: snapshot.gap.map(gap_to_dto),
        descriptor: descriptor_to_dto(snapshot.descriptor),
    }
}

fn frame_to_dto(shell_id: &str, frame: ShellOutputFrame) -> ShellOutputFrameDto {
    ShellOutputFrameDto {
        shell_id: shell_id.to_string(),
        sequence: frame.sequence,
        occurred_at: frame.occurred_at,
        stream: frame.stream.token(),
        data: frame.data,
    }
}

fn gap_to_dto(gap: ShellReplayGap) -> ShellReplayGapDto {
    ShellReplayGapDto {
        from_sequence: gap.from_sequence,
        to_sequence: gap.to_sequence,
        reason: gap.reason.as_str().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::domain::{
        shell_reason, SessionShellState, ShellForegroundProcessState, ShellRuntimeDescriptor,
        ShellStream,
    };

    fn descriptor(state: SessionShellState) -> SessionShellDescriptor {
        SessionShellDescriptor {
            shell_id: ShellId::parse("shell-1").expect("shell id"),
            session_id: "session-1".to_string(),
            seat_id: None,
            title: ShellTitle::parse("Shell 1").expect("title"),
            runtime: ShellRuntimeDescriptor::Native,
            state,
            created_at: "2026-08-22T09:00:00Z".to_string(),
            last_activity_at: "2026-08-22T09:00:05Z".to_string(),
            revision: 2,
            foreground_process: ShellForegroundProcessState::Unknown,
        }
    }

    /// An unknown foreground state has to survive the wire as itself. Serializing it as an absent
    /// field would let the close confirmation say "nothing is running" about a shell midway through
    /// a deploy.
    #[test]
    fn a_descriptor_keeps_an_unknown_foreground_state_as_a_value() {
        let dto = serde_json::to_value(descriptor_to_dto(descriptor(SessionShellState::Running)))
            .expect("descriptor");

        assert_eq!(dto["foregroundProcess"], "unknown");
        assert_eq!(dto["state"], "running");
        assert_eq!(dto["revision"], 2);
        assert!(dto.get("reason").is_none());
        assert!(dto.get("exitCode").is_none());
    }

    #[test]
    fn a_descriptor_carries_the_fact_its_state_actually_has() {
        let exited =
            serde_json::to_value(descriptor_to_dto(descriptor(SessionShellState::Exited {
                code: Some(1),
            })))
            .expect("exited");
        let failed =
            serde_json::to_value(descriptor_to_dto(descriptor(SessionShellState::Failed {
                reason: shell_reason("shell_process_launch_failed"),
            })))
            .expect("failed");

        assert_eq!(exited["exitCode"], 1);
        assert!(exited.get("reason").is_none());
        assert_eq!(failed["reason"], "shell_process_launch_failed");
        assert!(failed.get("exitCode").is_none());
    }

    /// Every replay frame names its Shell. The registry stores frames per Shell and does not repeat
    /// the id, but a view merging replay with live frames keys on it, and a frame that arrived
    /// without one would have to inherit the id of whatever request it came back on.
    #[test]
    fn replay_frames_carry_the_shell_they_belong_to() {
        let attachment = attachment_to_dto(ShellAttachSnapshot {
            attachment_id: ShellAttachmentId::parse("attach-1").expect("attachment id"),
            descriptor: descriptor(SessionShellState::Running),
            replay: vec![ShellOutputFrame {
                sequence: 4,
                occurred_at: "2026-08-22T09:00:04Z".to_string(),
                stream: ShellStream::Pty,
                data: "ready".to_string(),
            }],
            next_sequence: 5,
            gap: Some(ShellReplayGap {
                from_sequence: 1,
                to_sequence: 3,
                reason: shell_reason("shell_replay_evicted"),
            }),
        });

        let dto = serde_json::to_value(attachment).expect("attachment");
        assert_eq!(dto["attachmentId"], "attach-1");
        assert_eq!(dto["replay"][0]["shellId"], "shell-1");
        assert_eq!(dto["replay"][0]["sequence"], 4);
        assert_eq!(dto["nextSequence"], 5);
        assert_eq!(dto["gap"]["fromSequence"], 1);
        assert_eq!(dto["gap"]["reason"], "shell_replay_evicted");
    }

    /// A rename dialog the user opened and closed sends an empty string, and so does a client that
    /// omitted the field. Neither is a title, and rejecting either as invalid would turn a no-op
    /// into an error the user has to dismiss.
    #[test]
    fn blank_optional_text_is_absent_rather_than_invalid() {
        let request = create_request(CreateSessionShellInput {
            session_id: "session-1".to_string(),
            rows: 24,
            cols: 80,
            seat_id: Some("   ".to_string()),
            request_id: Some(String::new()),
            title: Some("  ".to_string()),
            working_directory: None,
        })
        .expect("request");

        assert_eq!(request.seat_id, None);
        assert_eq!(request.request_id, None);
        assert_eq!(request.title, None);
    }
}
