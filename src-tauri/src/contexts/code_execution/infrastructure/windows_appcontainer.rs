use crate::contexts::code_execution::application::{
    SandboxBackendCapabilities, SandboxBackendError, SandboxLaunchRequest, SandboxProcess,
};
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{FreeSid, PSID};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
    JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_JOB_TIME, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

use super::windows_sandbox_acl::grant_launch_access;
use super::windows_sandbox_process::{spawn_suspended, WindowsSandboxProcess};

const PROFILE_NAME: &str = "VaneHub.OnePiece.CodeExecution.v1";
static LAUNCH_SETUP: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn capabilities() -> SandboxBackendCapabilities {
    let appcontainer_available = AppContainerSid::probe();
    SandboxBackendCapabilities {
        restricted_identity: appcontainer_available,
        job_cpu_limit: true,
        job_memory_limit: true,
        job_process_limit: true,
        kill_process_tree: true,
        acl_confinement: appcontainer_available,
        network_denied: appcontainer_available,
    }
}

pub(super) fn launch(
    request: SandboxLaunchRequest,
) -> Result<Box<dyn SandboxProcess>, SandboxBackendError> {
    let setup = LAUNCH_SETUP.get_or_init(|| Mutex::new(()));
    let setup_guard = setup
        .lock()
        .map_err(|_| SandboxBackendError::IsolationUnavailable)?;
    let sid = AppContainerSid::acquire()?;
    grant_launch_access(&request.executable, &request.working_directory, sid.0)?;
    let job = create_limited_job(&request)?;
    let spawned = spawn_suspended(&request, sid.0)?;
    drop(setup_guard);
    if unsafe { AssignProcessToJobObject(job.0, spawned.process_handle()) } == 0 {
        return Err(SandboxBackendError::JobAssignmentFailed);
    }
    let spawned = spawned.resume()?;
    Ok(Box::new(WindowsSandboxProcess::new(job.take(), spawned)))
}

struct AppContainerSid(PSID);

impl AppContainerSid {
    fn probe() -> bool {
        let name = wide(std::ffi::OsStr::new(PROFILE_NAME));
        let mut sid = null_mut();
        let result = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut sid) };
        if !sid.is_null() {
            unsafe {
                FreeSid(sid);
            }
        }
        result >= 0
    }

    fn acquire() -> Result<Self, SandboxBackendError> {
        let name = wide(std::ffi::OsStr::new(PROFILE_NAME));
        let display = wide(std::ffi::OsStr::new("VaneHub OnePiece Code Execution"));
        let description = wide(std::ffi::OsStr::new("Networkless OnePiece code sandbox"));
        let mut created = null_mut();
        unsafe {
            let _ = CreateAppContainerProfile(
                name.as_ptr(),
                display.as_ptr(),
                description.as_ptr(),
                null(),
                0,
                &mut created,
            );
            if !created.is_null() {
                FreeSid(created);
            }
        }
        let mut sid = null_mut();
        let result = unsafe { DeriveAppContainerSidFromAppContainerName(name.as_ptr(), &mut sid) };
        if result < 0 || sid.is_null() {
            Err(SandboxBackendError::IsolationUnavailable)
        } else {
            Ok(Self(sid))
        }
    }
}

impl Drop for AppContainerSid {
    fn drop(&mut self) {
        unsafe {
            FreeSid(self.0);
        }
    }
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn take(mut self) -> HANDLE {
        let handle = self.0;
        self.0 = null_mut();
        handle
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

fn create_limited_job(request: &SandboxLaunchRequest) -> Result<OwnedHandle, SandboxBackendError> {
    let handle = unsafe { CreateJobObjectW(null(), null()) };
    if handle.is_null() {
        return Err(SandboxBackendError::JobSetupFailed);
    }
    let job = OwnedHandle(handle);
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_JOB_TIME
        | JOB_OBJECT_LIMIT_JOB_MEMORY
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
    limits.BasicLimitInformation.PerJobUserTimeLimit =
        i64::try_from(request.limits.cpu_time_ms.saturating_mul(10_000)).unwrap_or(i64::MAX);
    limits.BasicLimitInformation.ActiveProcessLimit = request.limits.process_count;
    limits.JobMemoryLimit = usize::try_from(request.limits.memory_bytes)
        .map_err(|_| SandboxBackendError::InvalidLaunch)?;
    let ok = unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast::<c_void>(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        Err(SandboxBackendError::JobSetupFailed)
    } else {
        Ok(job)
    }
}

fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
