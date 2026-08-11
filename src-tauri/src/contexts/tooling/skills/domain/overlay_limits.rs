#![cfg_attr(not(test), allow(dead_code))]

const KIBIBYTE: u64 = 1_024;
const MEBIBYTE: u64 = KIBIBYTE * KIBIBYTE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverlayLimits {
    pub(crate) maximum_instruction_characters: usize,
    pub(crate) maximum_mutations: usize,
    pub(crate) maximum_path_characters: usize,
    pub(crate) maximum_path_depth: usize,
    pub(crate) maximum_archive_entries: usize,
    pub(crate) maximum_supporting_file_bytes: u64,
    pub(crate) maximum_import_bytes: u64,
    pub(crate) maximum_expanded_import_bytes: u64,
    pub(crate) maximum_history_segment_bytes: u64,
}

pub(crate) const DEFAULT_OVERLAY_LIMITS: OverlayLimits = OverlayLimits {
    maximum_instruction_characters: 64 * 1_024,
    maximum_mutations: 256,
    maximum_path_characters: 240,
    maximum_path_depth: 8,
    maximum_archive_entries: 512,
    maximum_supporting_file_bytes: MEBIBYTE,
    maximum_import_bytes: 8 * MEBIBYTE,
    maximum_expanded_import_bytes: 32 * MEBIBYTE,
    maximum_history_segment_bytes: 4 * MEBIBYTE,
};
