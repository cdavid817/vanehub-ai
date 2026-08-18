use super::remote_command::{
    encode_owned_remote_command, encode_remote_command_probe, encode_remote_process_probe,
    encode_remote_process_signal, REMOTE_EXIT_PREFIX, REMOTE_PID_PREFIX,
};
use crate::contexts::agent_runtime::application::{
    AgentRunner, AgentSessionGateway, AgentSessionRunnerTarget, PreparedRunnerLaunch,
    RunnerCapabilities, RunnerError, RunnerErrorKind, RunnerEvent, RunnerHandle, RunnerInspection,
    RunnerKind, RunnerLaunchSpec, RunnerRecoveryMode, RunnerReference, RunnerSelection,
};
use crate::contexts::ssh_connections::api::{
    SshConnectionsApi, SshExecutionChannel, SshExecutionChannelEvent, SshExecutionLease,
    SshExecutionPoolHealth, SshExecutionPoolSnapshot, SshExecutionProfile, SshRuntimeError,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_PROBE_EVENTS: usize = 32;
const MAX_SPAWN_EVENTS: usize = 64;
const MAX_CONTROL_BUFFER_BYTES: usize = 64 * 1024;
const MAX_CANCEL_PROBES: usize = 2;
const MAX_CONTROL_EVENTS: usize = 16;

type SshRunnerResult<T> = Result<T, Box<SshRuntimeError>>;

trait SshSessionTargetPort: Send + Sync {
    fn current_target(&self, session_id: &str) -> Result<Option<AgentSessionRunnerTarget>, ()>;
}

struct PublishedSessionTarget {
    sessions: Arc<dyn AgentSessionGateway>,
}

impl SshSessionTargetPort for PublishedSessionTarget {
    fn current_target(&self, session_id: &str) -> Result<Option<AgentSessionRunnerTarget>, ()> {
        self.sessions
            .current_runner_target(session_id)
            .map_err(|_| ())
    }
}

trait SshRunnerChannelPort: Send + Sync {
    fn write(&self, content: &[u8]) -> SshRunnerResult<()>;
    fn next_event(&self) -> SshRunnerResult<Option<SshExecutionChannelEvent>>;
    fn close(&self) -> SshRunnerResult<()>;
}

trait SshRunnerLeasePort: Send + Sync {
    fn is_healthy(&self) -> bool;
    fn keepalive(&self) -> SshRunnerResult<()>;
    fn open_exec(&self, command: &[u8]) -> SshRunnerResult<Arc<dyn SshRunnerChannelPort>>;
}

trait SshRunnerGateway: Send + Sync {
    fn profile(&self, connection_id: &str) -> SshRunnerResult<SshExecutionProfile>;
    fn pool_snapshot(&self) -> Vec<SshExecutionPoolSnapshot>;
    fn acquire(
        &self,
        connection_id: &str,
        revision: i64,
    ) -> SshRunnerResult<Arc<dyn SshRunnerLeasePort>>;
}

struct PublishedSshGateway {
    api: SshConnectionsApi,
}

impl SshRunnerGateway for PublishedSshGateway {
    fn profile(&self, connection_id: &str) -> SshRunnerResult<SshExecutionProfile> {
        self.api.execution_profile(connection_id).map_err(Box::new)
    }

    fn pool_snapshot(&self) -> Vec<SshExecutionPoolSnapshot> {
        self.api.execution_pool_snapshot()
    }

    fn acquire(
        &self,
        connection_id: &str,
        revision: i64,
    ) -> SshRunnerResult<Arc<dyn SshRunnerLeasePort>> {
        tauri::async_runtime::block_on(self.api.acquire_execution(connection_id, revision))
            .map(|lease| Arc::new(PublishedLease { lease }) as Arc<dyn SshRunnerLeasePort>)
            .map_err(Box::new)
    }
}

struct PublishedLease {
    lease: SshExecutionLease,
}

impl SshRunnerLeasePort for PublishedLease {
    fn is_healthy(&self) -> bool {
        self.lease.is_healthy()
    }

    fn keepalive(&self) -> SshRunnerResult<()> {
        tauri::async_runtime::block_on(self.lease.keepalive()).map_err(Box::new)
    }

    fn open_exec(&self, command: &[u8]) -> SshRunnerResult<Arc<dyn SshRunnerChannelPort>> {
        tauri::async_runtime::block_on(self.lease.open_exec(command))
            .map(|channel| Arc::new(PublishedChannel { channel }) as Arc<dyn SshRunnerChannelPort>)
            .map_err(Box::new)
    }
}

struct PublishedChannel {
    channel: SshExecutionChannel,
}

impl SshRunnerChannelPort for PublishedChannel {
    fn write(&self, content: &[u8]) -> SshRunnerResult<()> {
        tauri::async_runtime::block_on(self.channel.write(content)).map_err(Box::new)
    }

    fn next_event(&self) -> SshRunnerResult<Option<SshExecutionChannelEvent>> {
        tauri::async_runtime::block_on(self.channel.next_event()).map_err(Box::new)
    }

    fn close(&self) -> SshRunnerResult<()> {
        tauri::async_runtime::block_on(self.channel.close()).map_err(Box::new)
    }
}

struct PreparedSshLaunch {
    lease: Arc<dyn SshRunnerLeasePort>,
    command: Vec<u8>,
}

struct SshProcess {
    lease: Arc<dyn SshRunnerLeasePort>,
    channel: Arc<dyn SshRunnerChannelPort>,
    remote_pid: u32,
    pending: VecDeque<RunnerEvent>,
    output_buffer: Vec<u8>,
    terminal: Option<RunnerInspection>,
}

pub(crate) struct SshRunner {
    sessions: Arc<dyn SshSessionTargetPort>,
    ssh: Arc<dyn SshRunnerGateway>,
    prepared: Mutex<HashMap<String, PreparedSshLaunch>>,
    handles: Mutex<HashMap<String, Arc<Mutex<SshProcess>>>>,
    ids: AtomicU64,
}

impl SshRunner {
    pub(crate) fn new(sessions: Arc<dyn AgentSessionGateway>, ssh: SshConnectionsApi) -> Self {
        Self {
            sessions: Arc::new(PublishedSessionTarget { sessions }),
            ssh: Arc::new(PublishedSshGateway { api: ssh }),
            prepared: Mutex::new(HashMap::new()),
            handles: Mutex::new(HashMap::new()),
            ids: AtomicU64::new(0),
        }
    }

    #[cfg(test)]
    fn with_ports(sessions: Arc<dyn SshSessionTargetPort>, ssh: Arc<dyn SshRunnerGateway>) -> Self {
        Self {
            sessions,
            ssh,
            prepared: Mutex::new(HashMap::new()),
            handles: Mutex::new(HashMap::new()),
            ids: AtomicU64::new(0),
        }
    }
}

impl AgentRunner for SshRunner {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Ssh
    }

    fn capabilities(&self) -> RunnerCapabilities {
        RunnerCapabilities {
            interactive_input: true,
            pty: true,
            cancellation: true,
            inspection: true,
            recovery: RunnerRecoveryMode::InspectOnly,
        }
    }

    fn prepare(
        &self,
        selection: &RunnerSelection,
        mut spec: RunnerLaunchSpec,
    ) -> Result<PreparedRunnerLaunch, RunnerError> {
        selection.validate()?;
        spec.validate_for(RunnerKind::Ssh)?;
        if selection.kind != RunnerKind::Ssh {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        let connection_id = selection
            .target_id
            .as_deref()
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::InvalidSelection))?;
        let revision = selection
            .target_revision
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::InvalidSelection))?;
        let session_id = spec
            .session_id
            .as_deref()
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::InvalidLaunch))?;
        let target = self
            .sessions
            .current_target(session_id)
            .map_err(|_| RunnerError::new(RunnerErrorKind::Preparation))?
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::InvalidSelection))?;
        if target.session_id != session_id
            || target.connection_id != connection_id
            || target.connection_revision != revision
        {
            return Err(RunnerError::new(RunnerErrorKind::AuthorityStale));
        }
        let profile = self
            .ssh
            .profile(connection_id)
            .map_err(|error| map_preparation_error(&error))?;
        if profile.connection_id != connection_id
            || profile.revision != revision
            || profile.host != target.host
            || profile.port != target.port
            || profile.user != target.user
        {
            return Err(RunnerError::new(RunnerErrorKind::AuthorityStale));
        }
        if !profile.host_trusted || !profile.credential_configured {
            return Err(RunnerError::new(RunnerErrorKind::Preparation));
        }
        if self.ssh.pool_snapshot().iter().any(|entry| {
            entry.connection_id == connection_id
                && entry.revision == revision
                && entry.health != SshExecutionPoolHealth::Healthy
        }) {
            return Err(RunnerError::new(RunnerErrorKind::Disconnected));
        }

        spec.cwd = Some(target.workspace_path);
        let command = encode_owned_remote_command(&spec)?.bytes;
        let lease = self
            .ssh
            .acquire(connection_id, revision)
            .map_err(|error| map_preparation_error(&error))?;
        if !lease.is_healthy() {
            return Err(RunnerError::new(RunnerErrorKind::Disconnected));
        }
        lease
            .keepalive()
            .map_err(|error| map_preparation_error(&error))?;
        probe_command(lease.as_ref(), &spec.executable)?;

        let preparation_id = format!(
            "ssh-prepared-{}",
            self.ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        self.prepared
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Preparation))?
            .insert(preparation_id.clone(), PreparedSshLaunch { lease, command });
        Ok(PreparedRunnerLaunch {
            reference: RunnerReference {
                kind: RunnerKind::Ssh,
                target_id: connection_id.to_string(),
                target_revision: Some(revision),
                recovery: RunnerRecoveryMode::InspectOnly,
                authority_witness: format!("ssh-authority:{connection_id}:{revision}"),
            },
            spec,
            preparation_id: Some(preparation_id),
            admission_id: None,
        })
    }

    fn spawn(&self, prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
        if prepared.reference.kind != RunnerKind::Ssh {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        let preparation_id = prepared
            .preparation_id
            .as_deref()
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::Preparation))?;
        let prepared_ssh = self
            .prepared
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?
            .remove(preparation_id)
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::Preparation))?;
        let channel = prepared_ssh
            .lease
            .open_exec(&prepared_ssh.command)
            .map_err(|error| map_spawn_error(&error))?;
        let mut process = SshProcess {
            lease: prepared_ssh.lease,
            channel: channel.clone(),
            remote_pid: 0,
            pending: VecDeque::new(),
            output_buffer: Vec::new(),
            terminal: None,
        };
        for _ in 0..MAX_SPAWN_EVENTS {
            let event = channel
                .next_event()
                .map_err(|error| map_spawn_error(&error))?;
            apply_channel_event(&mut process, event)?;
            if process.remote_pid > 0 {
                let handle_id = format!(
                    "ssh-handle-{}",
                    self.ids.fetch_add(1, Ordering::Relaxed) + 1
                );
                let process_reference = opaque_process_reference(
                    process.remote_pid,
                    &prepared.reference.target_id,
                    prepared.reference.target_revision,
                );
                self.handles
                    .lock()
                    .map_err(|_| RunnerError::new(RunnerErrorKind::Spawn))?
                    .insert(handle_id.clone(), Arc::new(Mutex::new(process)));
                return Ok(RunnerHandle {
                    id: handle_id.clone(),
                    reference: prepared.reference,
                    process_reference: Some(process_reference),
                });
            }
            if process.terminal.is_some() {
                break;
            }
        }
        let _ = channel.close();
        Err(RunnerError::new(RunnerErrorKind::Spawn))
    }

    fn send_input(&self, handle: &RunnerHandle, content: &[u8]) -> Result<(), RunnerError> {
        let channel = self
            .handle(handle)?
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Input))?
            .channel
            .clone();
        channel
            .write(content)
            .map_err(|_| RunnerError::new(RunnerErrorKind::Input))
    }

    fn next_event(&self, handle: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
        let process = self.handle(handle)?;
        let channel = {
            let mut state = process
                .lock()
                .map_err(|_| RunnerError::new(RunnerErrorKind::Disconnected))?;
            if let Some(event) = state.pending.pop_front() {
                return Ok(Some(event));
            }
            if state.terminal.is_some() {
                return Ok(None);
            }
            state.channel.clone()
        };
        let event = channel.next_event().map_err(|_| {
            if let Ok(mut state) = process.lock() {
                state.terminal = Some(RunnerInspection::Disconnected);
            }
            RunnerError::new(RunnerErrorKind::Disconnected)
        })?;
        let mut state = process
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Disconnected))?;
        apply_channel_event(&mut state, event)?;
        Ok(state.pending.pop_front())
    }

    fn cancel(&self, handle: &RunnerHandle) -> Result<bool, RunnerError> {
        let process = self.handle(handle)?;
        let (lease, channel, pid) = {
            let state = process
                .lock()
                .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
            if matches!(state.terminal, Some(RunnerInspection::Exited(_))) {
                return Ok(false);
            }
            (state.lease.clone(), state.channel.clone(), state.remote_pid)
        };
        if !remote_process_alive(lease.as_ref(), pid)? {
            return Ok(false);
        }
        run_control(lease.as_ref(), &encode_remote_process_signal(pid, false)?)?;
        let mut alive = false;
        for _ in 0..MAX_CANCEL_PROBES {
            alive = remote_process_alive(lease.as_ref(), pid)?;
            if !alive {
                break;
            }
            std::thread::yield_now();
        }
        if alive {
            run_control(lease.as_ref(), &encode_remote_process_signal(pid, true)?)?;
        }
        channel
            .close()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
        let mut state = process
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
        if state.terminal.is_none() {
            state.terminal = Some(RunnerInspection::Exited(None));
            state.pending.push_back(RunnerEvent::Exited(None));
        }
        Ok(true)
    }

    fn inspect(&self, handle: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
        let process = self.handle(handle)?;
        let state = process
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Inspection))?;
        Ok(state.terminal.clone().unwrap_or(RunnerInspection::Running))
    }

    fn cleanup(&self, handle: &RunnerHandle) -> Result<(), RunnerError> {
        let process = self
            .handles
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))?
            .remove(&handle.id);
        let Some(process) = process else {
            return Ok(());
        };
        let channel = process
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))?
            .channel
            .clone();
        channel
            .close()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cleanup))
    }

    fn recover(
        &self,
        reference: &RunnerReference,
        process_reference: Option<&str>,
    ) -> Result<RunnerInspection, RunnerError> {
        if reference.kind != RunnerKind::Ssh
            || reference.recovery != RunnerRecoveryMode::InspectOnly
        {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        let revision = reference
            .target_revision
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::AuthorityStale))?;
        let pid = decode_process_reference(
            process_reference.ok_or_else(|| RunnerError::new(RunnerErrorKind::Inspection))?,
            &reference.target_id,
            revision,
        )?;
        let profile = self
            .ssh
            .profile(&reference.target_id)
            .map_err(|error| map_recovery_error(&error))?;
        if profile.revision != revision || !profile.host_trusted || !profile.credential_configured {
            return Err(RunnerError::new(RunnerErrorKind::AuthorityStale));
        }
        let lease = self
            .ssh
            .acquire(&reference.target_id, revision)
            .map_err(|error| map_recovery_error(&error))?;
        lease
            .keepalive()
            .map_err(|error| map_recovery_error(&error))?;
        if remote_process_alive(lease.as_ref(), pid)
            .map_err(|_| RunnerError::new(RunnerErrorKind::ReconnectExhausted))?
        {
            Ok(RunnerInspection::Running)
        } else {
            Ok(RunnerInspection::Exited(None))
        }
    }
}

impl SshRunner {
    fn handle(&self, handle: &RunnerHandle) -> Result<Arc<Mutex<SshProcess>>, RunnerError> {
        if handle.reference.kind != RunnerKind::Ssh {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        self.handles
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Inspection))?
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::Inspection))
    }
}

fn apply_channel_event(
    process: &mut SshProcess,
    event: Option<SshExecutionChannelEvent>,
) -> Result<(), RunnerError> {
    match event {
        Some(SshExecutionChannelEvent::Output(content)) => append_stdout(process, &content)?,
        Some(SshExecutionChannelEvent::ExtendedOutput { content, .. }) => {
            process.pending.push_back(RunnerEvent::Stderr(content));
        }
        Some(SshExecutionChannelEvent::ExitStatus(status)) => {
            let status = i32::try_from(status).ok();
            process.terminal = Some(RunnerInspection::Exited(status));
            process.pending.push_back(RunnerEvent::Exited(status));
        }
        Some(SshExecutionChannelEvent::ExitSignal(_)) => {
            process.terminal = Some(RunnerInspection::Exited(None));
            process.pending.push_back(RunnerEvent::Exited(None));
        }
        Some(SshExecutionChannelEvent::Eof) => flush_output(process),
        Some(SshExecutionChannelEvent::Closed) | None => {
            flush_output(process);
            if process.terminal.is_none() {
                process.terminal = Some(RunnerInspection::Disconnected);
                process.pending.push_back(RunnerEvent::Disconnected);
            }
        }
    }
    Ok(())
}

fn append_stdout(process: &mut SshProcess, content: &[u8]) -> Result<(), RunnerError> {
    process.output_buffer.extend_from_slice(content);
    if process.output_buffer.len() > MAX_CONTROL_BUFFER_BYTES {
        return Err(RunnerError::new(RunnerErrorKind::ResourceExhausted));
    }
    while let Some(index) = process.output_buffer.iter().position(|byte| *byte == b'\n') {
        let line = process.output_buffer.drain(..=index).collect::<Vec<_>>();
        let text = String::from_utf8_lossy(&line);
        if let Some(value) = text.strip_prefix(REMOTE_PID_PREFIX) {
            process.remote_pid = value
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|pid| *pid > 1)
                .ok_or_else(|| RunnerError::new(RunnerErrorKind::Spawn))?;
        } else if let Some(value) = text.strip_prefix(REMOTE_EXIT_PREFIX) {
            let status = value.trim().parse::<i32>().ok();
            process.terminal = Some(RunnerInspection::Exited(status));
            process.pending.push_back(RunnerEvent::Exited(status));
        } else {
            process.pending.push_back(RunnerEvent::Stdout(line));
        }
    }
    Ok(())
}

fn flush_output(process: &mut SshProcess) {
    if !process.output_buffer.is_empty() {
        process
            .pending
            .push_back(RunnerEvent::Stdout(std::mem::take(
                &mut process.output_buffer,
            )));
    }
}

fn map_spawn_error(error: &SshRuntimeError) -> RunnerError {
    let kind = match error {
        SshRuntimeError::PoolAtCapacity => RunnerErrorKind::ResourceExhausted,
        SshRuntimeError::TransportClosed | SshRuntimeError::ChannelFailed => {
            RunnerErrorKind::Disconnected
        }
        _ => RunnerErrorKind::Spawn,
    };
    RunnerError::new(kind)
}

fn opaque_process_reference(pid: u32, connection_id: &str, revision: Option<i64>) -> String {
    URL_SAFE_NO_PAD.encode(format!(
        "v1:{connection_id}:{}:{pid}",
        revision.unwrap_or_default()
    ))
}

fn decode_process_reference(
    encoded: &str,
    connection_id: &str,
    revision: i64,
) -> Result<u32, RunnerError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(|| RunnerError::new(RunnerErrorKind::Inspection))?;
    let mut parts = decoded.split(':');
    if parts.next() != Some("v1")
        || parts.next() != Some(connection_id)
        || parts.next().and_then(|value| value.parse::<i64>().ok()) != Some(revision)
    {
        return Err(RunnerError::new(RunnerErrorKind::AuthorityStale));
    }
    let pid = parts
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|pid| *pid > 1)
        .ok_or_else(|| RunnerError::new(RunnerErrorKind::Inspection))?;
    if parts.next().is_some() {
        return Err(RunnerError::new(RunnerErrorKind::Inspection));
    }
    Ok(pid)
}

fn map_recovery_error(error: &SshRuntimeError) -> RunnerError {
    let kind = match error {
        SshRuntimeError::StaleProfile
        | SshRuntimeError::HostKeyVerificationFailed
        | SshRuntimeError::MissingAuthenticationMaterial => RunnerErrorKind::AuthorityStale,
        SshRuntimeError::PoolAtCapacity => RunnerErrorKind::ResourceExhausted,
        _ => RunnerErrorKind::ReconnectExhausted,
    };
    RunnerError::new(kind)
}

fn remote_process_alive(lease: &dyn SshRunnerLeasePort, pid: u32) -> Result<bool, RunnerError> {
    run_control(lease, &encode_remote_process_probe(pid)?)
}

fn run_control(lease: &dyn SshRunnerLeasePort, command: &[u8]) -> Result<bool, RunnerError> {
    let channel = lease
        .open_exec(command)
        .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
    let mut success = false;
    for _ in 0..MAX_CONTROL_EVENTS {
        match channel
            .next_event()
            .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?
        {
            Some(SshExecutionChannelEvent::ExitStatus(0)) => {
                success = true;
                break;
            }
            Some(SshExecutionChannelEvent::ExitStatus(_))
            | Some(SshExecutionChannelEvent::Closed)
            | None => break,
            Some(_) => {}
        }
    }
    channel
        .close()
        .map_err(|_| RunnerError::new(RunnerErrorKind::Cancellation))?;
    Ok(success)
}

fn probe_command(lease: &dyn SshRunnerLeasePort, executable: &str) -> Result<(), RunnerError> {
    let channel = lease
        .open_exec(&encode_remote_command_probe(executable)?)
        .map_err(|error| map_preparation_error(&error))?;
    let mut succeeded = false;
    for _ in 0..MAX_PROBE_EVENTS {
        match channel
            .next_event()
            .map_err(|error| map_preparation_error(&error))?
        {
            Some(SshExecutionChannelEvent::ExitStatus(0)) => succeeded = true,
            Some(SshExecutionChannelEvent::ExitStatus(_)) => break,
            Some(SshExecutionChannelEvent::Closed) | None => break,
            Some(_) => {}
        }
    }
    channel
        .close()
        .map_err(|error| map_preparation_error(&error))?;
    if succeeded {
        Ok(())
    } else {
        Err(RunnerError::new(RunnerErrorKind::Preparation))
    }
}

fn map_preparation_error(error: &SshRuntimeError) -> RunnerError {
    let kind = match error {
        SshRuntimeError::StaleProfile => RunnerErrorKind::AuthorityStale,
        SshRuntimeError::PoolAtCapacity => RunnerErrorKind::ResourceExhausted,
        SshRuntimeError::TransportClosed | SshRuntimeError::ChannelFailed => {
            RunnerErrorKind::Disconnected
        }
        _ => RunnerErrorKind::Preparation,
    };
    RunnerError::new(kind)
}

#[cfg(test)]
#[path = "ssh_runner_tests.rs"]
mod tests;
