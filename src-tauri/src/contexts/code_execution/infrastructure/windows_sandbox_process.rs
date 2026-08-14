use crate::contexts::code_execution::application::{
    SandboxBackendError, SandboxLaunchRequest, SandboxProcess, SandboxProcessObservation,
};
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{GetLastError, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::JobObjects::TerminateJobObject;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, ResumeThread, TerminateProcess, WaitForSingleObject,
    CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
    PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

use super::windows_sandbox_support::{
    command_line, environment_block, join_reader, peak_memory, process_cpu_ms, reader, wide,
    AttributeList, OwnedHandle, Pipe,
};

pub(super) struct SpawnedProcess {
    process: Option<OwnedHandle>,
    thread: Option<OwnedHandle>,
    stdout: Option<OwnedHandle>,
    stderr: Option<OwnedHandle>,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl SpawnedProcess {
    pub(super) fn process_handle(&self) -> HANDLE {
        self.process.as_ref().map_or(null_mut(), |value| value.0)
    }

    pub(super) fn resume(mut self) -> Result<Self, SandboxBackendError> {
        let thread = self
            .thread
            .as_ref()
            .ok_or(SandboxBackendError::SpawnFailed)?;
        if unsafe { ResumeThread(thread.0) } == u32::MAX {
            return Err(SandboxBackendError::ResumeFailed);
        }
        self.thread.take();
        Ok(self)
    }
}

impl Drop for SpawnedProcess {
    fn drop(&mut self) {
        if let Some(process) = &self.process {
            unsafe {
                TerminateProcess(process.0, 1);
            }
        }
    }
}

pub(super) struct WindowsSandboxProcess {
    job: OwnedHandle,
    process: OwnedHandle,
    stdout: Option<JoinHandle<Vec<u8>>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
    completed: bool,
}

unsafe impl Send for WindowsSandboxProcess {}

impl WindowsSandboxProcess {
    pub(super) fn new(job: HANDLE, mut spawned: SpawnedProcess) -> Self {
        let stdout = reader(
            spawned.stdout.take().map(OwnedHandle::take),
            spawned.stdout_limit,
        );
        let stderr = reader(
            spawned.stderr.take().map(OwnedHandle::take),
            spawned.stderr_limit,
        );
        let process = spawned
            .process
            .take()
            .unwrap_or_else(|| OwnedHandle(null_mut()));
        Self {
            job: OwnedHandle(job),
            process,
            stdout,
            stderr,
            completed: false,
        }
    }

    fn observation(&mut self) -> Result<SandboxProcessObservation, SandboxBackendError> {
        let mut exit_code = 0_u32;
        if unsafe { GetExitCodeProcess(self.process.0, &mut exit_code) } == 0 {
            return Err(SandboxBackendError::WaitFailed);
        }
        let stdout = join_reader(self.stdout.take())?;
        let stderr = join_reader(self.stderr.take())?;
        self.completed = true;
        Ok(SandboxProcessObservation {
            exit_code: exit_code as i32,
            stdout,
            stderr,
            cpu_time_ms: process_cpu_ms(self.process.0),
            peak_memory_bytes: peak_memory(self.job.0),
        })
    }
}

impl SandboxProcess for WindowsSandboxProcess {
    fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<SandboxProcessObservation>, SandboxBackendError> {
        if self.completed {
            return Err(SandboxBackendError::WaitFailed);
        }
        let timeout = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(u128::from(u32::MAX)) as u32;
        match unsafe { WaitForSingleObject(self.process.0, timeout) } {
            WAIT_OBJECT_0 => self.observation().map(Some),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(SandboxBackendError::WaitFailed),
        }
    }

    fn terminate_tree(&mut self, timeout: Duration) -> Result<(), SandboxBackendError> {
        if unsafe { TerminateJobObject(self.job.0, 1) } == 0 {
            return Err(SandboxBackendError::TerminationFailed);
        }
        let millis = timeout.as_millis().min(u128::from(u32::MAX)) as u32;
        if unsafe { WaitForSingleObject(self.process.0, millis) } == WAIT_OBJECT_0 {
            self.completed = true;
            Ok(())
        } else {
            Err(SandboxBackendError::TerminationFailed)
        }
    }
}

impl Drop for WindowsSandboxProcess {
    fn drop(&mut self) {
        if !self.completed {
            unsafe {
                TerminateJobObject(self.job.0, 1);
            }
        }
    }
}

pub(super) fn spawn_suspended(
    request: &SandboxLaunchRequest,
    sid: *mut c_void,
) -> Result<SpawnedProcess, SandboxBackendError> {
    let stdin = Pipe::new(true)?;
    let stdout = Pipe::new(false)?;
    let stderr = Pipe::new(false)?;
    drop(stdin.write);
    let inherited = [stdin.read.0, stdout.write.0, stderr.write.0];
    let mut attributes = AttributeList::new(sid, &inherited)?;
    let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = stdin.read.0;
    startup.StartupInfo.hStdOutput = stdout.write.0;
    startup.StartupInfo.hStdError = stderr.write.0;
    startup.lpAttributeList = attributes.pointer();
    let application = wide(request.executable.as_os_str());
    let mut command = command_line(request);
    let current_directory = wide(request.working_directory.as_os_str());
    let environment = environment_block(request);
    let mut process_info: PROCESS_INFORMATION = unsafe { zeroed() };
    let flags = CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT;
    let ok = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            1,
            flags,
            environment.as_ptr().cast(),
            current_directory.as_ptr(),
            (&raw const startup.StartupInfo),
            &mut process_info,
        )
    };
    if ok == 0 {
        return Err(SandboxBackendError::ProcessCreationFailed(unsafe {
            GetLastError()
        }));
    }
    drop(stdout.write);
    drop(stderr.write);
    Ok(SpawnedProcess {
        process: Some(OwnedHandle(process_info.hProcess)),
        thread: Some(OwnedHandle(process_info.hThread)),
        stdout: Some(stdout.read),
        stderr: Some(stderr.read),
        stdout_limit: usize::try_from(request.limits.stdout_bytes).unwrap_or(usize::MAX),
        stderr_limit: usize::try_from(request.limits.stderr_bytes).unwrap_or(usize::MAX),
    })
}
