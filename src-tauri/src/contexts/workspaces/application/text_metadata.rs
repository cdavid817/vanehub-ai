//! What a preview can say about a file besides its contents.
//!
//! Two facts, both of which change what a reader does. A byte order mark is invisible and breaks
//! shell scripts, JSON parsers, and `#!` lines — a preview that renders the file perfectly while
//! the tool chain refuses it leaves nowhere to look. Mixed line endings are the other: a diff full
//! of whole-file changes has one cause far more often than any other, and it is this.
//!
//! Derived here rather than in either provider, so the two cannot disagree. The remote helper sends
//! back decoded text and this side classifies it, which means the classification is the same
//! function for a file on this machine and a file on a host across the world.

/// How a text file was encoded, as far as this application is concerned.
///
/// Two variants, and deliberately only two. Anything that does not decode as UTF-8 is reported as
/// binary rather than as some other encoding: this application does not transcode, and naming an
/// encoding it cannot read would be offering a preview that never arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextEncoding {
    Utf8,
    /// UTF-8 with a byte order mark. Invisible on screen and load-bearing everywhere else.
    Utf8Bom,
}

impl TextEncoding {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf8Bom => "utf-8-bom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewlineStyle {
    Lf,
    Crlf,
    /// Both, in one file. The one worth surfacing: it is what turns an ordinary edit into a diff
    /// that claims every line changed.
    Mixed,
    /// No line ending at all — a single line, with or without content.
    None,
}

impl NewlineStyle {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Lf => "lf",
            Self::Crlf => "crlf",
            Self::Mixed => "mixed",
            Self::None => "none",
        }
    }
}

/// The byte order mark, as it appears at the start of a UTF-8 file.
const UTF8_BOM: &str = "\u{feff}";

pub(crate) fn detect_encoding(content: &str) -> TextEncoding {
    if content.starts_with(UTF8_BOM) {
        TextEncoding::Utf8Bom
    } else {
        TextEncoding::Utf8
    }
}

/// Which line endings a file uses.
///
/// Counted rather than sampled from the first one. A file whose first thousand lines are `\n` and
/// whose last is `\r\n` is mixed, and that last line is exactly the one somebody appended by hand.
pub(crate) fn detect_newline(content: &str) -> NewlineStyle {
    let carriage_returns = content.matches("\r\n").count();
    // `\n` that are not part of a `\r\n`. Counting all of them would make every CRLF file look
    // mixed, since each of its line endings contains one.
    let bare_line_feeds = content.matches('\n').count() - carriage_returns;
    match (carriage_returns, bare_line_feeds) {
        (0, 0) => NewlineStyle::None,
        (0, _) => NewlineStyle::Lf,
        (_, 0) => NewlineStyle::Crlf,
        _ => NewlineStyle::Mixed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_order_mark_is_reported_because_it_is_invisible() {
        assert_eq!(detect_encoding("\u{feff}#!/bin/sh\n").token(), "utf-8-bom");
        assert_eq!(detect_encoding("#!/bin/sh\n").token(), "utf-8");
    }

    #[test]
    fn a_crlf_file_is_not_reported_as_mixed() {
        // Every `\r\n` contains an `\n`. Counting line feeds naively would make every file written
        // on Windows look like it had two conventions in it.
        assert_eq!(detect_newline("a\r\nb\r\n").token(), "crlf");
    }

    #[test]
    fn one_stray_ending_makes_a_file_mixed() {
        // The case worth surfacing: a hand-appended last line is what turns an ordinary edit into a
        // diff that claims every line changed.
        assert_eq!(detect_newline("a\nb\nc\r\n").token(), "mixed");
    }

    #[test]
    fn a_single_line_has_no_ending_rather_than_a_default() {
        // `none` rather than the platform's convention: the file does not say, and picking one for
        // it would be reporting a fact nobody established.
        assert_eq!(detect_newline("no trailing newline").token(), "none");
        assert_eq!(detect_newline("").token(), "none");
    }

    #[test]
    fn plain_line_feeds_are_reported_as_such() {
        assert_eq!(detect_newline("a\nb\n").token(), "lf");
    }
}
