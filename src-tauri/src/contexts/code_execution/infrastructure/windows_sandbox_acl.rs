use crate::contexts::code_execution::application::SandboxBackendError;
use std::mem::size_of;
use std::path::Path;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
    GRANT_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::{
    AclSizeInformation, EqualSid, GetAce, GetAclInformation, ACCESS_ALLOWED_ACE,
    ACL_SIZE_INFORMATION, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, OBJECT_INHERIT_ACE,
    PSID,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
};

use super::windows_sandbox_support::wide;

pub(super) fn grant_launch_access(
    executable: &Path,
    work: &Path,
    sid: PSID,
) -> Result<(), SandboxBackendError> {
    let runtime = executable
        .parent()
        .ok_or(SandboxBackendError::InvalidLaunch)?;
    if !is_windows_system_runtime(runtime) {
        grant_tree(runtime, sid, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE)?;
        grant_path(
            executable,
            sid,
            FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
            false,
        )?;
    }
    let root = work.parent().ok_or(SandboxBackendError::InvalidLaunch)?;
    grant_tree(root, sid, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE)?;
    grant_tree(&root.join("inputs"), sid, FILE_GENERIC_READ)?;
    grant_tree(
        work,
        sid,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE,
    )?;
    grant_tree(
        &root.join("outputs"),
        sid,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
    )
}

fn is_windows_system_runtime(runtime: &Path) -> bool {
    let windows = std::env::var_os("SystemRoot")
        .or_else(|| std::env::var_os("WINDIR"))
        .unwrap_or_else(|| "C:\\Windows".into());
    runtime.starts_with(Path::new(&windows))
}

fn grant_tree(path: &Path, sid: PSID, access: u32) -> Result<(), SandboxBackendError> {
    grant_path(path, sid, access, true)
}

fn grant_path(
    path: &Path,
    sid: PSID,
    access: u32,
    inherit: bool,
) -> Result<(), SandboxBackendError> {
    let path = wide(path.as_os_str());
    let mut old_acl = null_mut();
    let mut descriptor = null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut old_acl,
            null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Err(SandboxBackendError::AclSetupFailed);
    }
    if acl_grants_access(old_acl, sid, access, inherit) {
        unsafe { LocalFree(descriptor) };
        return Ok(());
    }
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: access,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: if inherit {
            OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE
        } else {
            0
        },
        Trustee: TRUSTEE_W {
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: sid.cast::<u16>(),
            ..Default::default()
        },
    };
    let mut new_acl = null_mut();
    let acl_status = unsafe { SetEntriesInAclW(1, &raw const entry, old_acl, &mut new_acl) };
    let set_status = if acl_status == 0 {
        unsafe {
            SetNamedSecurityInfoW(
                path.as_ptr() as *mut u16,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                new_acl,
                std::ptr::null(),
            )
        }
    } else {
        acl_status
    };
    unsafe {
        if !new_acl.is_null() {
            LocalFree(new_acl.cast());
        }
        LocalFree(descriptor);
    }
    if set_status == 0 {
        Ok(())
    } else {
        Err(SandboxBackendError::AclSetupFailed)
    }
}

fn acl_grants_access(
    acl: *mut windows_sys::Win32::Security::ACL,
    sid: PSID,
    access: u32,
    inherit: bool,
) -> bool {
    if acl.is_null() {
        return false;
    }
    let mut info = ACL_SIZE_INFORMATION::default();
    if unsafe {
        GetAclInformation(
            acl,
            (&raw mut info).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    } == 0
    {
        return false;
    }
    (0..info.AceCount).any(|index| {
        let mut raw_ace = null_mut();
        if unsafe { GetAce(acl, index, &mut raw_ace) } == 0 || raw_ace.is_null() {
            return false;
        }
        let ace = unsafe { &*raw_ace.cast::<ACCESS_ALLOWED_ACE>() };
        let inheritance = (OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE) as u8;
        ace.Header.AceType == 0
            && ace.Mask & access == access
            && (!inherit || ace.Header.AceFlags & inheritance == inheritance)
            && unsafe { EqualSid((&raw const ace.SidStart).cast_mut().cast(), sid) } != 0
    })
}
