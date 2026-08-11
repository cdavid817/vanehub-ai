use super::DEFAULT_OVERLAY_LIMITS;

#[test]
fn storage_limits_match_the_governance_contract() {
    let limits = DEFAULT_OVERLAY_LIMITS;

    assert_eq!(limits.maximum_supporting_file_bytes, 1_048_576);
    assert_eq!(limits.maximum_import_bytes, 8_388_608);
    assert_eq!(limits.maximum_history_segment_bytes, 4_194_304);
}

#[test]
fn every_overlay_input_dimension_has_a_finite_non_zero_limit() {
    let limits = DEFAULT_OVERLAY_LIMITS;

    assert_eq!(limits.maximum_instruction_characters, 65_536);
    assert_eq!(limits.maximum_mutations, 256);
    assert_eq!(limits.maximum_path_characters, 240);
    assert_eq!(limits.maximum_path_depth, 8);
    assert_eq!(limits.maximum_archive_entries, 512);
    assert_eq!(limits.maximum_expanded_import_bytes, 33_554_432);
    assert!(limits.maximum_expanded_import_bytes > limits.maximum_import_bytes);
}

#[test]
fn defaults_are_copyable_so_all_validation_stages_share_one_snapshot() {
    let preview_limits = DEFAULT_OVERLAY_LIMITS;
    let commit_limits = preview_limits;

    assert_eq!(preview_limits, commit_limits);
}
