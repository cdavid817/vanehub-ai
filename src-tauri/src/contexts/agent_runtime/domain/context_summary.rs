#![allow(dead_code)]

use sha2::{Digest, Sha256};

pub(crate) const STRUCTURED_SUMMARY_VERSION: &str = "onepiece-continuation-summary-v1";
pub(crate) const STRUCTURED_SUMMARY_MAX_CHARACTERS: usize = 12_000;
const REQUIRED_SECTIONS: [(&str, &str); 8] = [
    ("primary-intent", "PRIMARY INTENT"),
    ("constraints", "TECHNICAL CONSTRAINTS"),
    ("decisions", "DECISIONS"),
    ("files-code", "FILES AND CODE AREAS"),
    ("errors-fixes", "ERRORS AND FIXES"),
    ("completed", "COMPLETED WORK"),
    ("pending", "PENDING WORK"),
    ("next-action", "IMMEDIATE NEXT ACTION"),
];

pub(crate) const STRUCTURED_SUMMARY_PROMPT: &str = r#"Produce a continuation summary using exactly these headings, in this order:
## PRIMARY INTENT
## TECHNICAL CONSTRAINTS
## DECISIONS
## FILES AND CODE AREAS
## ERRORS AND FIXES
## COMPLETED WORK
## PENDING WORK
## IMMEDIATE NEXT ACTION

Preserve identifiers and exact user constraints where required for continuation. Do not call tools, include hidden thinking, or add headings."#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructuredSummaryFailure {
    Empty,
    Oversized,
    MissingSection,
    DuplicateSection,
    OutOfOrderSection,
    EmptySection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructuredSummarySectionEvidence {
    pub(crate) section: &'static str,
    pub(crate) characters: u32,
    pub(crate) fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructuredSummaryEvidence {
    pub(crate) version: &'static str,
    pub(crate) characters: u32,
    pub(crate) sections: Vec<StructuredSummarySectionEvidence>,
}

pub(crate) fn parse_structured_summary(
    value: &str,
) -> Result<StructuredSummaryEvidence, StructuredSummaryFailure> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StructuredSummaryFailure::Empty);
    }
    let characters = value.chars().count();
    if characters > STRUCTURED_SUMMARY_MAX_CHARACTERS {
        return Err(StructuredSummaryFailure::Oversized);
    }

    let headings: Vec<_> = value
        .lines()
        .enumerate()
        .filter_map(|(line, text)| {
            text.strip_prefix("## ")
                .map(|heading| (line, heading.trim()))
        })
        .collect();
    for (_, heading) in REQUIRED_SECTIONS {
        let count = headings
            .iter()
            .filter(|(_, value)| *value == heading)
            .count();
        if count == 0 {
            return Err(StructuredSummaryFailure::MissingSection);
        }
        if count > 1 {
            return Err(StructuredSummaryFailure::DuplicateSection);
        }
    }
    if headings.len() != REQUIRED_SECTIONS.len()
        || headings
            .iter()
            .zip(REQUIRED_SECTIONS)
            .any(|((_, actual), (_, expected))| actual != &expected)
    {
        return Err(StructuredSummaryFailure::OutOfOrderSection);
    }

    let lines: Vec<_> = value.lines().collect();
    let mut sections = Vec::with_capacity(REQUIRED_SECTIONS.len());
    for (position, ((line, _), (section, _))) in headings.iter().zip(REQUIRED_SECTIONS).enumerate()
    {
        let end = headings
            .get(position + 1)
            .map(|(next, _)| *next)
            .unwrap_or(lines.len());
        let body = lines[line + 1..end].join("\n");
        let body = body.trim();
        if body.is_empty() {
            return Err(StructuredSummaryFailure::EmptySection);
        }
        sections.push(StructuredSummarySectionEvidence {
            section,
            characters: body.chars().count().min(u32::MAX as usize) as u32,
            fingerprint: fingerprint(body),
        });
    }
    Ok(StructuredSummaryEvidence {
        version: STRUCTURED_SUMMARY_VERSION,
        characters: characters.min(u32::MAX as usize) as u32,
        sections,
    })
}

fn fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
