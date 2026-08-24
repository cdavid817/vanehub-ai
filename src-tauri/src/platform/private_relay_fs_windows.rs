use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    GetTokenInformation, SetFileSecurityW, TokenUser, DACL_SECURITY_INFORMATION,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub(super) fn restrict_to_current_user(path: &Path) -> io::Result<()> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(windows_error)?;
    let result = apply_current_user_dacl(path, token);
    let _ = unsafe { CloseHandle(token) };
    result
}

fn apply_current_user_dacl(path: &Path, token: HANDLE) -> io::Result<()> {
    let sid = current_user_sid(token)?;
    apply_sddl(path, &format!("D:P(A;;FA;;;{sid})"))
}

fn current_user_sid(token: HANDLE) -> io::Result<String> {
    let mut required = 0;
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
    if required == 0 {
        return Err(io::Error::last_os_error());
    }
    let word = std::mem::size_of::<usize>();
    let mut token_buffer = vec![0usize; (required as usize).div_ceil(word)];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(token_buffer.as_mut_ptr().cast()),
            required,
            &mut required,
        )
    }
    .map_err(windows_error)?;
    let token_user = unsafe { &*token_buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut sid_text = PWSTR::null();
    unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut sid_text) }.map_err(windows_error)?;
    let sid = unsafe { sid_text.to_string() }
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()));
    unsafe { LocalFree(Some(HLOCAL(sid_text.0.cast()))) };
    sid
}

fn apply_sddl(path: &Path, sddl: &str) -> io::Result<()> {
    let sddl_wide = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl_wide.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .map_err(windows_error)?;
    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let applied = unsafe {
        SetFileSecurityW(
            PCWSTR(path_wide.as_ptr()),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
        .ok()
    }
    .map_err(windows_error);
    unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    applied
}

fn windows_error(error: windows::core::Error) -> io::Error {
    io::Error::other(error.to_string())
}

/// Applies an arbitrary DACL, for negative tests only.
///
/// Exposed so a test can build a *wrong* DACL -- an extra principal, an unprotected descriptor,
/// a bad mask -- and require the contract check to reject it. A structural check that nothing
/// has ever falsified is the same class of thing as the string comparison it replaced.
#[cfg(test)]
pub(super) fn apply_sddl_for_tests(path: &Path, sddl: &str) -> io::Result<()> {
    apply_sddl(path, sddl)
}
