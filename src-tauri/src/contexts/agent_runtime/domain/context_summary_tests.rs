use super::*;

fn valid_summary() -> String {
    [
        ("PRIMARY INTENT", "Continue the task."),
        ("TECHNICAL CONSTRAINTS", "Keep protocol valid."),
        ("DECISIONS", "Use a neutral plan."),
        ("FILES AND CODE AREAS", "domain/context_summary.rs"),
        ("ERRORS AND FIXES", "No open error."),
        ("COMPLETED WORK", "Planner completed."),
        ("PENDING WORK", "Runtime integration."),
        ("IMMEDIATE NEXT ACTION", "Build candidate."),
    ]
    .into_iter()
    .map(|(heading, body)| format!("## {heading}\n{body}"))
    .collect::<Vec<_>>()
    .join("\n")
}

#[test]
fn accepts_all_required_sections_and_returns_content_free_evidence() {
    let source = valid_summary();
    let evidence = parse_structured_summary(&source).expect("summary");
    assert_eq!(evidence.version, STRUCTURED_SUMMARY_VERSION);
    assert_eq!(evidence.sections.len(), 8);
    let diagnostic = format!("{evidence:?}");
    assert!(!diagnostic.contains("Continue the task"));
    assert!(!diagnostic.contains("Runtime integration"));
    assert!(evidence
        .sections
        .iter()
        .all(|section| { section.fingerprint.len() == 24 && section.characters > 0 }));
}

#[test]
fn rejects_empty_missing_duplicate_reordered_and_empty_sections() {
    assert_eq!(
        parse_structured_summary(""),
        Err(StructuredSummaryFailure::Empty)
    );
    let valid = valid_summary();
    assert_eq!(
        parse_structured_summary(&valid.replace("## PENDING WORK\nRuntime integration.\n", "")),
        Err(StructuredSummaryFailure::MissingSection)
    );
    assert_eq!(
        parse_structured_summary(&format!("{valid}\n## PRIMARY INTENT\nduplicate")),
        Err(StructuredSummaryFailure::DuplicateSection)
    );
    assert_eq!(
        parse_structured_summary(
            &valid
                .replacen("PRIMARY INTENT", "TEMP", 1)
                .replacen("DECISIONS", "PRIMARY INTENT", 1)
                .replacen("TEMP", "DECISIONS", 1)
        ),
        Err(StructuredSummaryFailure::OutOfOrderSection)
    );
    assert_eq!(
        parse_structured_summary(
            &valid.replace("## DECISIONS\nUse a neutral plan.", "## DECISIONS")
        ),
        Err(StructuredSummaryFailure::EmptySection)
    );
}

#[test]
fn rejects_oversized_output_without_retaining_source() {
    let oversized = "x".repeat(STRUCTURED_SUMMARY_MAX_CHARACTERS + 1);
    let error = parse_structured_summary(&oversized).expect_err("oversized");
    assert_eq!(error, StructuredSummaryFailure::Oversized);
    assert!(!format!("{error:?}").contains(&oversized));
}
