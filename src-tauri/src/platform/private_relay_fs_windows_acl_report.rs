//! A structural reading of a Windows object's DACL, for the relay privacy contract.
//!
//! The contract in `mcp-client-management` is about *access*: only the current user may reach a
//! directory that is about to hold secrets. That is a statement about an owner, principals,
//! masks, inheritance and protection. The check it replaced compared the descriptor's SDDL
//! rendering as a string, which is a shadow of the contract -- it passed because one Windows
//! build happened to render one descriptor one way, and when it stopped passing it said only
//! `false`.
//!
//! Everything here is read-only. Nothing in this module changes an ACL.

use std::fmt::Write as _;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    AclSizeInformation, GetAce, GetAclInformation, GetFileSecurityW, GetSecurityDescriptorControl,
    GetSecurityDescriptorDacl, GetSecurityDescriptorOwner, ACCESS_ALLOWED_ACE, ACCESS_DENIED_ACE,
    ACE_HEADER, ACL, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, SE_DACL_AUTO_INHERITED, SE_DACL_PRESENT, SE_DACL_PROTECTED,
};

/// `FILE_ALL_ACCESS` for a file or directory, which is what `FA` denotes in SDDL.
pub(super) const FILE_ALL_ACCESS_MASK: u32 = 0x001F_01FF;

const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const ACCESS_DENIED_ACE_TYPE: u8 = 1;
const INHERITED_ACE_FLAG: u8 = 0x10;

/// One entry, read in the order Windows stores it.
///
/// Order is preserved rather than normalised because Windows evaluates entries in sequence and
/// stops at the first match: a deny placed after an allow does not deny. Sorting before
/// comparison would hide exactly the defect canonical ordering exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AceReading {
    pub(super) index: u32,
    /// `None` for an ACE type this reader does not model. Reported rather than skipped: an
    /// unmodelled entry is still an entry, and silently dropping it would let one through.
    pub(super) allowed: Option<bool>,
    pub(super) ace_type: u8,
    pub(super) inherited: bool,
    pub(super) inheritance_flags: u8,
    pub(super) sid: String,
    pub(super) mask: u32,
}

impl AceReading {
    fn describe(&self) -> String {
        let kind = match self.allowed {
            Some(true) => "allow".to_string(),
            Some(false) => "deny".to_string(),
            None => format!("unmodelled(type={})", self.ace_type),
        };
        format!(
            "#{} {} {} sid={} mask={:#010x} ({}) inheritance_flags={:#04x}",
            self.index,
            kind,
            if self.inherited {
                "inherited"
            } else {
                "explicit"
            },
            self.sid,
            self.mask,
            normalised_mask(self.mask),
            self.inheritance_flags,
        )
    }
}

/// The whole structural reading of one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DaclReading {
    pub(super) current_user_sid: String,
    pub(super) owner_sid: Option<String>,
    /// `false` means a NULL DACL, which grants everyone full control. An *empty* DACL denies
    /// everyone. The two render alike as "no entries" and are opposites, so they are separate
    /// fields rather than one count.
    pub(super) dacl_present: bool,
    pub(super) protected: bool,
    pub(super) auto_inherited: bool,
    pub(super) aces: Vec<AceReading>,
    pub(super) raw_sddl: Option<String>,
}

impl DaclReading {
    /// Whether this reading satisfies the contract: present, protected, owned by the current
    /// user, and exactly one explicit allow-all entry for the current user.
    pub(super) fn satisfies_private_current_user_contract(&self) -> bool {
        self.violations().is_empty()
    }

    /// Every way this reading departs from the contract, each as its own sentence. Reported as
    /// a list rather than a first-failure so one run shows the whole picture.
    pub(super) fn violations(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if !self.dacl_present {
            violations.push(
                "DACL is NULL, which grants full control to everyone rather than to nobody"
                    .to_string(),
            );
            // With no DACL there are no entries to describe; everything below would be noise.
            return violations;
        }
        if !self.protected {
            violations.push(
                "DACL is not protected, so inheritance from the parent can widen it later"
                    .to_string(),
            );
        }
        match &self.owner_sid {
            Some(owner) if owner == &self.current_user_sid => {}
            Some(owner) => violations.push(format!(
                "owner is {owner}, not the current user {}; an owner may rewrite the DACL \
                 regardless of its contents",
                self.current_user_sid
            )),
            None => violations.push("owner could not be read".to_string()),
        }
        if self.aces.len() != 1 {
            violations.push(format!(
                "expected exactly one access-allowed entry, found {}",
                self.aces.len()
            ));
        }
        for ace in &self.aces {
            if ace.inherited {
                violations.push(format!(
                    "entry #{} is inherited; a protected DACL must carry none",
                    ace.index
                ));
            }
            if ace.allowed != Some(true) {
                violations.push(format!(
                    "entry #{} is not an access-allowed entry",
                    ace.index
                ));
            }
            if ace.sid != self.current_user_sid {
                violations.push(format!(
                    "entry #{} names {}, which is not the current user {}",
                    ace.index, ace.sid, self.current_user_sid
                ));
            }
            if ace.mask != FILE_ALL_ACCESS_MASK {
                violations.push(format!(
                    "entry #{} grants mask {:#010x} ({}), expected {:#010x} (FILE_ALL_ACCESS)",
                    ace.index,
                    ace.mask,
                    normalised_mask(ace.mask),
                    FILE_ALL_ACCESS_MASK
                ));
            }
            if ace.inheritance_flags != 0 {
                violations.push(format!(
                    "entry #{} carries inheritance flags {:#04x}, expected none",
                    ace.index, ace.inheritance_flags
                ));
            }
        }
        violations
    }

    /// States what was found, what was expected, and the difference -- so a reader does not have
    /// to derive any of the three.
    ///
    /// Printed on success as well as failure, and the heading says which. A report that always
    /// reads "not satisfied" would be misleading in the one case where a baseline is most
    /// useful: comparing a machine where the contract holds against one where it does not.
    pub(super) fn describe(&self, label: &str, path: &Path) -> String {
        let violations = self.violations();
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{label} DACL contract {} for {}",
            if violations.is_empty() {
                "SATISFIED"
            } else {
                "NOT SATISFIED"
            },
            path.display()
        );
        let _ = writeln!(out, "  current user SID : {}", self.current_user_sid);
        let _ = writeln!(
            out,
            "  owner SID        : {}",
            self.owner_sid.as_deref().unwrap_or("<unreadable>")
        );
        let _ = writeln!(
            out,
            "  DACL             : {}",
            if self.dacl_present {
                "present"
            } else {
                "NULL (grants everyone full control)"
            }
        );
        let _ = writeln!(
            out,
            "  protected        : {}   auto-inherited : {}",
            self.protected, self.auto_inherited
        );
        let _ = writeln!(out, "  entries ({}, in stored order):", self.aces.len());
        if self.aces.is_empty() {
            let _ = writeln!(out, "    <none>");
        }
        for ace in &self.aces {
            let _ = writeln!(out, "    {}", ace.describe());
        }
        let _ = writeln!(out, "  expected contract:");
        let _ = writeln!(
            out,
            "    DACL present, protected, owner = current user, exactly one explicit"
        );
        let _ = writeln!(
            out,
            "    access-allowed entry for {} with mask {:#010x} (FILE_ALL_ACCESS)",
            self.current_user_sid, FILE_ALL_ACCESS_MASK
        );
        let _ = writeln!(out, "    and no inheritance flags");
        let _ = writeln!(out, "  differences:");
        if violations.is_empty() {
            let _ = writeln!(out, "    <none>");
        }
        for violation in &violations {
            let _ = writeln!(out, "    - {violation}");
        }
        // Supplementary only. This is the field that must never be the assertion.
        let _ = writeln!(
            out,
            "  raw SDDL (diagnostic only): {}",
            self.raw_sddl.as_deref().unwrap_or("<unreadable>")
        );
        out
    }
}

/// Renders a mask as the named rights it contains, so `0x1f01ff` and `FA` can be compared as
/// access rather than as text.
pub(super) fn normalised_mask(mask: u32) -> String {
    const NAMED: &[(u32, &str)] = &[
        (0x1000_0000, "GENERIC_ALL"),
        (0x2000_0000, "GENERIC_EXECUTE"),
        (0x4000_0000, "GENERIC_WRITE"),
        (0x8000_0000, "GENERIC_READ"),
        (0x0010_0000, "SYNCHRONIZE"),
        (0x0008_0000, "WRITE_OWNER"),
        (0x0004_0000, "WRITE_DAC"),
        (0x0002_0000, "READ_CONTROL"),
        (0x0001_0000, "DELETE"),
        (0x0000_0100, "FILE_WRITE_ATTRIBUTES"),
        (0x0000_0080, "FILE_READ_ATTRIBUTES"),
        (0x0000_0040, "FILE_DELETE_CHILD"),
        (0x0000_0020, "FILE_EXECUTE"),
        (0x0000_0010, "FILE_WRITE_EA"),
        (0x0000_0008, "FILE_READ_EA"),
        (0x0000_0004, "FILE_APPEND_DATA"),
        (0x0000_0002, "FILE_WRITE_DATA"),
        (0x0000_0001, "FILE_READ_DATA"),
    ];
    if mask == FILE_ALL_ACCESS_MASK {
        return "FILE_ALL_ACCESS".to_string();
    }
    let mut parts = Vec::new();
    let mut remaining = mask;
    for (bit, name) in NAMED {
        if mask & bit == *bit {
            parts.push(*name);
            remaining &= !bit;
        }
    }
    if remaining != 0 {
        parts.push("…");
    }
    if parts.is_empty() {
        return "<none>".to_string();
    }
    parts.join("|")
}

fn windows_error(error: windows::core::Error) -> io::Error {
    io::Error::other(error.to_string())
}

fn sid_to_string(sid: PSID) -> io::Result<String> {
    let mut text = PWSTR::null();
    unsafe { ConvertSidToStringSidW(sid, &mut text) }.map_err(windows_error)?;
    let value = unsafe { text.to_string() }
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()));
    unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
    value
}

/// Reads owner and DACL for one path.
pub(super) fn read_dacl(path: &Path, current_user_sid: String) -> io::Result<DaclReading> {
    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let information = DACL_SECURITY_INFORMATION.0 | OWNER_SECURITY_INFORMATION.0;
    let mut required = 0;
    let _ = unsafe {
        GetFileSecurityW(
            PCWSTR(path_wide.as_ptr()),
            information,
            None,
            0,
            &mut required,
        )
    };
    if required == 0 {
        return Err(io::Error::last_os_error());
    }
    let word = std::mem::size_of::<usize>();
    let mut buffer = vec![0usize; (required as usize).div_ceil(word)];
    let descriptor = PSECURITY_DESCRIPTOR(buffer.as_mut_ptr().cast());
    unsafe {
        GetFileSecurityW(
            PCWSTR(path_wide.as_ptr()),
            information,
            Some(descriptor),
            required,
            &mut required,
        )
        .ok()
    }
    .map_err(windows_error)?;

    let mut control_bits = 0u16;
    let mut revision = 0u32;
    unsafe { GetSecurityDescriptorControl(descriptor, &mut control_bits, &mut revision) }
        .map_err(windows_error)?;
    let protected = control_bits & SE_DACL_PROTECTED.0 != 0;
    let auto_inherited = control_bits & SE_DACL_AUTO_INHERITED.0 != 0;
    let control_says_present = control_bits & SE_DACL_PRESENT.0 != 0;

    let mut owner_sid_ptr = PSID::default();
    let mut owner_defaulted = windows::core::BOOL::default();
    let owner_sid =
        unsafe { GetSecurityDescriptorOwner(descriptor, &mut owner_sid_ptr, &mut owner_defaulted) }
            .ok()
            .and_then(|()| {
                if owner_sid_ptr.0.is_null() {
                    None
                } else {
                    sid_to_string(owner_sid_ptr).ok()
                }
            });

    let mut dacl_ptr: *mut ACL = std::ptr::null_mut();
    let mut dacl_present = windows::core::BOOL::default();
    let mut dacl_defaulted = windows::core::BOOL::default();
    unsafe {
        GetSecurityDescriptorDacl(
            descriptor,
            &mut dacl_present,
            &mut dacl_ptr,
            &mut dacl_defaulted,
        )
    }
    .map_err(windows_error)?;

    // A present flag with a null pointer is the NULL DACL: everyone, full control.
    let present = control_says_present && dacl_present.as_bool() && !dacl_ptr.is_null();
    let mut aces = Vec::new();
    if present {
        let mut size = ACL_SIZE_INFORMATION::default();
        unsafe {
            GetAclInformation(
                dacl_ptr,
                std::ptr::addr_of_mut!(size).cast(),
                u32::try_from(std::mem::size_of::<ACL_SIZE_INFORMATION>()).unwrap_or(0),
                AclSizeInformation,
            )
        }
        .map_err(windows_error)?;
        for index in 0..size.AceCount {
            let mut ace_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            unsafe { GetAce(dacl_ptr, index, &mut ace_ptr) }.map_err(windows_error)?;
            if ace_ptr.is_null() {
                continue;
            }
            let header = unsafe { *ace_ptr.cast::<ACE_HEADER>() };
            let allowed = match header.AceType {
                ACCESS_ALLOWED_ACE_TYPE => Some(true),
                ACCESS_DENIED_ACE_TYPE => Some(false),
                _ => None,
            };
            // Allowed and denied ACEs share a layout: header, mask, then the SID inline.
            let (mask, sid) = match allowed {
                Some(true) => {
                    let ace = unsafe { &*ace_ptr.cast::<ACCESS_ALLOWED_ACE>() };
                    let sid_ptr = PSID(std::ptr::addr_of!(ace.SidStart) as *mut core::ffi::c_void);
                    (ace.Mask, sid_to_string(sid_ptr)?)
                }
                Some(false) => {
                    let ace = unsafe { &*ace_ptr.cast::<ACCESS_DENIED_ACE>() };
                    let sid_ptr = PSID(std::ptr::addr_of!(ace.SidStart) as *mut core::ffi::c_void);
                    (ace.Mask, sid_to_string(sid_ptr)?)
                }
                None => (0, "<unmodelled ACE type>".to_string()),
            };
            aces.push(AceReading {
                index,
                allowed,
                ace_type: header.AceType,
                inherited: header.AceFlags & INHERITED_ACE_FLAG != 0,
                inheritance_flags: header.AceFlags & !INHERITED_ACE_FLAG,
                sid,
                mask,
            });
        }
    }

    let mut sddl_ptr = PWSTR::null();
    let raw_sddl = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut sddl_ptr,
            None,
        )
    }
    .ok()
    .and_then(|()| {
        let text = unsafe { sddl_ptr.to_string() }.ok();
        unsafe { LocalFree(Some(HLOCAL(sddl_ptr.0.cast()))) };
        text
    });

    Ok(DaclReading {
        current_user_sid,
        owner_sid,
        dacl_present: present,
        protected,
        auto_inherited,
        aces,
        raw_sddl,
    })
}

/// The current process token's user SID.
pub(super) fn current_process_user_sid() -> io::Result<String> {
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(windows_error)?;
    let mut required = 0;
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
    if required == 0 {
        let error = io::Error::last_os_error();
        let _ = unsafe { CloseHandle(token) };
        return Err(error);
    }
    let word = std::mem::size_of::<usize>();
    let mut buffer = vec![0usize; (required as usize).div_ceil(word)];
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            required,
            &mut required,
        )
    }
    .map_err(windows_error);
    let sid = read.and_then(|()| {
        let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        sid_to_string(token_user.User.Sid)
    });
    let _ = unsafe { CloseHandle(token) };
    sid
}
