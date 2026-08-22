// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! One unambiguous byte encoding, used everywhere this context needs a value to have an identity.
//!
//! Two things depend on it and must not drift apart: the manifest digest a witness binds a
//! confirmation to, and the payload a publisher signs. If the second were written a second time,
//! the two encodings could diverge, and a signature would then attest to something subtly other
//! than what the installer compared against.
//!
//! The rules:
//!
//! * **Every string is length-prefixed.** Concatenating fields without that would let
//!   `["ab", "c"]` and `["a", "bc"]` produce identical bytes, and two different inputs sharing an
//!   encoding is precisely the failure both callers exist to prevent.
//! * **Sets are sorted, sequences are not.** Source order is meaningless for a set of origins and
//!   load-bearing for a command line. Sorting the second would make `--force` and `--dry-run`
//!   interchangeable.
//! * **Absent and present-but-empty encode differently.** They are different states.

#[derive(Default)]
pub(crate) struct Canonical {
    buffer: Vec<u8>,
}

impl Canonical {
    /// A field marker. Length-prefixed like everything else so a tag cannot be confused with the
    /// value before it.
    pub(crate) fn tag(&mut self, tag: &str) {
        self.text(tag);
    }

    pub(crate) fn text(&mut self, value: &str) {
        self.buffer
            .extend_from_slice(value.len().to_string().as_bytes());
        self.buffer.push(b':');
        self.buffer.extend_from_slice(value.as_bytes());
        self.buffer.push(b';');
    }

    /// Absent and present-but-empty are different states, so they encode differently.
    pub(crate) fn optional(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.text("some");
                self.text(value);
            }
            None => self.text("none"),
        }
    }

    /// Order-insensitive. Sorted after rendering so the encoding, not the source, decides.
    pub(crate) fn set<I: IntoIterator<Item = String>>(&mut self, items: I) {
        let mut rendered: Vec<String> = items.into_iter().collect();
        rendered.sort_unstable();
        self.text(&rendered.len().to_string());
        for item in rendered {
            self.text(&item);
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }
}

/// Joins parts unambiguously, for a value that is itself a record inside a set.
pub(crate) fn join(parts: &[&str]) -> String {
    let mut joined = String::new();
    for part in parts {
        joined.push_str(&part.len().to_string());
        joined.push(':');
        joined.push_str(part);
        joined.push(';');
    }
    joined
}

/// Order-significant rendering.
pub(crate) fn sequence(items: &[String]) -> String {
    join(&items.iter().map(String::as_str).collect::<Vec<_>>())
}

/// Order-insensitive rendering.
pub(crate) fn sorted(items: &[String]) -> String {
    let mut ordered: Vec<&str> = items.iter().map(String::as_str).collect();
    ordered.sort_unstable();
    join(&ordered)
}

pub(crate) const fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
