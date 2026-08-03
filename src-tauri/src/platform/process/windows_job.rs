use process_wrap::std::{ChildWrapper, CommandWrap, CommandWrapper};
use std::io::{Error, Result};
use std::os::windows::{io::AsRawHandle, process::CommandExt};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectAssociateCompletionPortInformation,
    JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    JOBOBJECT_ASSOCIATE_COMPLETION_PORT, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::SystemServices::JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO;
use windows::Win32::System::Threading::{
    GetProcessId, OpenThread, ResumeThread, CREATE_SUSPENDED, INFINITE, THREAD_SUSPEND_RESUME,
};
use windows::Win32::System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus, OVERLAPPED};

#[derive(Debug, Clone, Copy)]
pub(super) struct KillOnCloseJobObject {
    inject_failure: bool,
}

impl KillOnCloseJobObject {
    pub(super) const fn new() -> Self {
        Self {
            inject_failure: false,
        }
    }

    #[cfg(test)]
    const fn failing() -> Self {
        Self {
            inject_failure: true,
        }
    }
}

impl CommandWrapper for KillOnCloseJobObject {
    fn pre_spawn(&mut self, command: &mut Command, _core: &CommandWrap) -> Result<()> {
        command.creation_flags(CREATE_SUSPENDED.0);
        Ok(())
    }

    fn wrap_child(
        &mut self,
        mut child: Box<dyn ChildWrapper>,
        _core: &CommandWrap,
    ) -> Result<Box<dyn ChildWrapper>> {
        let result = self.contain(child.as_ref());
        match result {
            Ok(job) => Ok(Box::new(KillOnCloseJobChild { child, job })),
            Err(error) => {
                let _ = child.kill();
                Err(error)
            }
        }
    }
}

impl KillOnCloseJobObject {
    fn contain(&self, child: &dyn ChildWrapper) -> Result<JobPort> {
        if self.inject_failure {
            return Err(Error::other(
                "injected Windows Job Object containment failure",
            ));
        }
        let process = HANDLE(child.inner_child().as_raw_handle() as _);
        let job = JobHandle(unsafe { CreateJobObjectW(None, None) }.map_err(Error::other)?);
        let completion =
            JobHandle(unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, None, 0, 1) }?);
        let association = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
            CompletionKey: job.0 .0 as _,
            CompletionPort: completion.0,
        };
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectAssociateCompletionPortInformation,
                &association as *const _ as _,
                std::mem::size_of_val(&association)
                    .try_into()
                    .map_err(Error::other)?,
            )
        }
        .map_err(Error::other)?;
        let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &information as *const _ as _,
                std::mem::size_of_val(&information)
                    .try_into()
                    .map_err(Error::other)?,
            )
        }
        .map_err(Error::other)?;
        unsafe { AssignProcessToJobObject(job.0, process) }.map_err(Error::other)?;
        resume_process_threads(process)?;
        Ok(JobPort { job, completion })
    }
}

#[derive(Debug)]
struct KillOnCloseJobChild {
    child: Box<dyn ChildWrapper>,
    job: JobPort,
}

impl ChildWrapper for KillOnCloseJobChild {
    fn inner(&self) -> &dyn ChildWrapper {
        self.child.inner()
    }

    fn inner_mut(&mut self) -> &mut dyn ChildWrapper {
        self.child.inner_mut()
    }

    fn into_inner(self: Box<Self>) -> Box<dyn ChildWrapper> {
        let Self { child, job } = *self;
        std::mem::forget(job);
        child.into_inner()
    }

    fn start_kill(&mut self) -> Result<()> {
        unsafe { TerminateJobObject(self.job.job.0, 1) }.map_err(Error::other)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        if wait_for_job(self.job.completion.0, Some(Duration::ZERO))? {
            self.child.try_wait()
        } else {
            Ok(None)
        }
    }

    fn wait(&mut self) -> Result<ExitStatus> {
        let status = self.child.wait()?;
        wait_for_job(self.job.completion.0, None)?;
        Ok(status)
    }
}

#[derive(Debug)]
struct JobPort {
    job: JobHandle,
    completion: JobHandle,
}

#[derive(Debug)]
struct JobHandle(HANDLE);

unsafe impl Send for JobHandle {}
unsafe impl Sync for JobHandle {}

impl Drop for JobHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn wait_for_job(completion: HANDLE, timeout: Option<Duration>) -> Result<bool> {
    let deadline = timeout.map(|duration| Instant::now() + duration);
    loop {
        let milliseconds = deadline.map_or(INFINITE, |deadline| {
            deadline
                .saturating_duration_since(Instant::now())
                .as_millis()
                .try_into()
                .unwrap_or(INFINITE)
        });
        let mut code = 0;
        let mut key = 0;
        let mut overlapped = std::ptr::null_mut::<OVERLAPPED>();
        let result = unsafe {
            GetQueuedCompletionStatus(
                completion,
                &mut code,
                &mut key,
                &mut overlapped,
                milliseconds,
            )
        };
        if result.is_err() && overlapped.is_null() && timeout.is_some() {
            return Ok(false);
        }
        result.map_err(Error::other)?;
        if code == JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO {
            return Ok(true);
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Ok(false);
        }
    }
}

fn resume_process_threads(process: HANDLE) -> Result<()> {
    let process_id = unsafe { GetProcessId(process) };
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }?;
    let result = resume_snapshot_threads(process_id, snapshot);
    unsafe { CloseHandle(snapshot) }.map_err(Error::other)?;
    result
}

fn resume_snapshot_threads(process_id: u32, snapshot: HANDLE) -> Result<()> {
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut result = unsafe { Thread32First(snapshot, &mut entry) };
    while result.is_ok() {
        if entry.th32OwnerProcessID == process_id {
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }?;
            let resume_result = unsafe { ResumeThread(thread) };
            unsafe { CloseHandle(thread) }.map_err(Error::other)?;
            if resume_result == u32::MAX {
                return Err(Error::last_os_error());
            }
        }
        result = unsafe { Thread32Next(snapshot, &mut entry) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use process_wrap::std::CommandWrap;
    use std::fs;
    use std::thread;

    #[test]
    fn containment_failure_kills_and_reaps_the_suspended_child() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let marker = directory.path().join("fixture-ran");
        let executable = std::env::current_exe().expect("test executable");
        let mut command = Command::new(executable);
        command.args([
            "--ignored",
            "--exact",
            "platform::process::windows_job::tests::containment_failure_fixture",
            "--nocapture",
        ]);
        command.env("VANEHUB_CONTAINMENT_FAILURE_MARKER", &marker);
        let mut command = CommandWrap::from(command);
        command.wrap(KillOnCloseJobObject::failing());

        assert!(command.spawn().is_err());
        thread::sleep(Duration::from_millis(100));
        assert!(!marker.exists());
    }

    #[test]
    #[ignore = "spawned only by the Windows containment failure test"]
    fn containment_failure_fixture() {
        if let Some(marker) = std::env::var_os("VANEHUB_CONTAINMENT_FAILURE_MARKER") {
            fs::write(marker, b"ran").expect("marker");
        }
        thread::sleep(Duration::from_secs(30));
    }
}
