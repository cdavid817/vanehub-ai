use crate::contexts::code_execution::application::{SandboxBackendError, SandboxLaunchRequest};
use std::ffi::c_void;
use std::fs::File;
use std::io::Read;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::ptr::{null, null_mut};
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::{
    CloseHandle, SetHandleInformation, FILETIME, HANDLE, HANDLE_FLAG_INHERIT,
};
use windows_sys::Win32::Security::{SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES};
use windows_sys::Win32::System::JobObjects::{
    JobObjectExtendedLimitInformation, QueryInformationJobObject,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    DeleteProcThreadAttributeList, GetProcessTimes, InitializeProcThreadAttributeList,
    UpdateProcThreadAttribute, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
};

pub(super) struct Pipe {
    pub(super) read: OwnedHandle,
    pub(super) write: OwnedHandle,
}

impl Pipe {
    pub(super) fn new(child_reads: bool) -> Result<Self, SandboxBackendError> {
        let security = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1,
        };
        let mut read = null_mut();
        let mut write = null_mut();
        if unsafe { CreatePipe(&mut read, &mut write, &raw const security, 0) } == 0 {
            return Err(SandboxBackendError::SpawnFailed);
        }
        let parent_handle = if child_reads { write } else { read };
        if unsafe { SetHandleInformation(parent_handle, HANDLE_FLAG_INHERIT, 0) } == 0 {
            unsafe {
                CloseHandle(read);
                CloseHandle(write);
            }
            return Err(SandboxBackendError::SpawnFailed);
        }
        Ok(Self {
            read: OwnedHandle(read),
            write: OwnedHandle(write),
        })
    }
}

pub(super) struct AttributeList {
    storage: Vec<usize>,
    security: Box<SECURITY_CAPABILITIES>,
}

impl AttributeList {
    pub(super) fn new(sid: *mut c_void, handles: &[HANDLE]) -> Result<Self, SandboxBackendError> {
        let mut bytes = 0_usize;
        unsafe { InitializeProcThreadAttributeList(null_mut(), 2, 0, &mut bytes) };
        if bytes == 0 {
            return Err(SandboxBackendError::IsolationUnavailable);
        }
        let mut value = Self {
            storage: vec![0; bytes.div_ceil(size_of::<usize>())],
            security: Box::new(SECURITY_CAPABILITIES {
                AppContainerSid: sid,
                Capabilities: null_mut(),
                CapabilityCount: 0,
                Reserved: 0,
            }),
        };
        if unsafe { InitializeProcThreadAttributeList(value.pointer(), 2, 0, &mut bytes) } == 0 {
            return Err(SandboxBackendError::IsolationUnavailable);
        }
        let security_ok = unsafe {
            UpdateProcThreadAttribute(
                value.pointer(),
                0,
                PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
                (&raw const *value.security).cast(),
                size_of::<SECURITY_CAPABILITIES>(),
                null_mut(),
                null(),
            )
        };
        let handles_ok = unsafe {
            UpdateProcThreadAttribute(
                value.pointer(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr().cast(),
                std::mem::size_of_val(handles),
                null_mut(),
                null(),
            )
        };
        if security_ok == 0 || handles_ok == 0 {
            Err(SandboxBackendError::IsolationUnavailable)
        } else {
            Ok(value)
        }
    }

    pub(super) fn pointer(&mut self) -> *mut c_void {
        self.storage.as_mut_ptr().cast()
    }
}

impl Drop for AttributeList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.pointer()) };
    }
}

pub(super) struct OwnedHandle(pub(super) HANDLE);

impl OwnedHandle {
    pub(super) fn take(mut self) -> HANDLE {
        let handle = self.0;
        self.0 = null_mut();
        handle
    }
}

unsafe impl Send for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

pub(super) fn reader(handle: Option<HANDLE>, limit: usize) -> Option<JoinHandle<Vec<u8>>> {
    handle.map(|handle| {
        let raw = handle as usize;
        std::thread::spawn(move || {
            let mut file = unsafe { File::from_raw_handle(raw as RawHandle) };
            let mut output = Vec::with_capacity(limit.min(64 * 1024));
            let mut buffer = [0_u8; 8192];
            while let Ok(count) = file.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                let remaining = limit.saturating_add(1).saturating_sub(output.len());
                output.extend_from_slice(&buffer[..count.min(remaining)]);
            }
            output
        })
    })
}

pub(super) fn join_reader(
    reader: Option<JoinHandle<Vec<u8>>>,
) -> Result<Vec<u8>, SandboxBackendError> {
    reader
        .ok_or(SandboxBackendError::WaitFailed)?
        .join()
        .map_err(|_| SandboxBackendError::WaitFailed)
}

pub(super) fn process_cpu_ms(process: HANDLE) -> Option<u64> {
    let mut creation: FILETIME = unsafe { zeroed() };
    let mut exit: FILETIME = unsafe { zeroed() };
    let mut kernel: FILETIME = unsafe { zeroed() };
    let mut user: FILETIME = unsafe { zeroed() };
    if unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let ticks = filetime_ticks(kernel).saturating_add(filetime_ticks(user));
    Some(ticks / 10_000)
}

fn filetime_ticks(value: FILETIME) -> u64 {
    (u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
}

pub(super) fn peak_memory(job: HANDLE) -> Option<u64> {
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
    let ok = unsafe {
        QueryInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            (&raw mut info).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            null_mut(),
        )
    };
    (ok != 0).then_some(info.PeakJobMemoryUsed as u64)
}

pub(super) fn command_line(request: &SandboxLaunchRequest) -> Vec<u16> {
    let mut values = Vec::with_capacity(request.arguments.len() + 1);
    values.push(
        request
            .executable
            .as_os_str()
            .to_string_lossy()
            .into_owned(),
    );
    values.extend(request.arguments.iter().cloned());
    wide(std::ffi::OsStr::new(
        &values
            .iter()
            .map(|value| quote_argument(value))
            .collect::<Vec<_>>()
            .join(" "),
    ))
}

fn quote_argument(value: &str) -> String {
    if !value
        .chars()
        .any(|character| character.is_whitespace() || character == '"')
    {
        return value.to_owned();
    }
    let mut output = String::from("\"");
    let mut slashes = 0_usize;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
        } else {
            output.push_str(&"\\".repeat(if character == '"' {
                slashes * 2 + 1
            } else {
                slashes
            }));
            slashes = 0;
            output.push(character);
        }
    }
    output.push_str(&"\\".repeat(slashes * 2));
    output.push('"');
    output
}

pub(super) fn environment_block(request: &SandboxLaunchRequest) -> Vec<u16> {
    let mut block = Vec::new();
    for (key, value) in &request.environment {
        let key = match key.as_str() {
            "COMSPEC" => "ComSpec",
            "PATH" => "Path",
            "SYSTEMDRIVE" => "SystemDrive",
            "SYSTEMROOT" => "SystemRoot",
            "WINDIR" => "windir",
            _ => key,
        };
        block.extend(std::ffi::OsStr::new(&format!("{key}={value}")).encode_wide());
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    block
}

pub(super) fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}
