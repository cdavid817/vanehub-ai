//! What an archive entry may be called.
//!
//! An archive entry name is not an operating-system path. ZIP defines it as a `/`-separated
//! string that has to mean the same thing on every machine that opens the archive, so the rules
//! here work on the raw string and are identical on every platform. `Path::components()` is
//! deliberately not used: on Unix it treats a backslash as an ordinary filename character, so
//! `..\..\etc` would survive component analysis on a Linux runner and escape on Windows.
//!
//! Two neighbours answer different questions and are not interchangeable with this one:
//!
//! * `crate::platform::filesystem::validate_relative` validates a *workspace* path the user typed,
//!   where OS semantics are the point and `.` segments are ordinary.
//! * `extension_platform::domain::PortablePackagePath` validates a path a manifest *declares*, and
//!   is stricter still — Windows reserved names, alternate data streams, trailing dots and spaces,
//!   control characters, and Unicode collisions. It lives in a domain layer that may not reach out
//!   to `crate::platform`, which is why the rule is stated twice rather than shared.
//!
//! This check is the mechanical floor every archive consumer needs, not the whole policy. It
//! refuses traversal, separators, and hidden segments; it does not refuse a NUL byte, a Windows
//! reserved name, or a trailing dot. A consumer that writes entries out under those names needs
//! the stricter rule as well.

/// True when `name` is a relative, `/`-separated archive entry name with no traversal, no hidden
/// segment, and no character that changes meaning across platforms.
///
/// A single trailing `/` is allowed and ignored, because that is how ZIP spells a directory.
pub(crate) fn is_safe_archive_entry_path(name: &str) -> bool {
    let name = name.strip_suffix('/').unwrap_or(name);
    !name.is_empty()
        && !name.starts_with(['/', '\\'])
        && !name.contains('\\')
        && !name.contains(':')
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.starts_with('.'))
}
