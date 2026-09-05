#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ActivityReadOrderKey {
    pub(crate) committed_at_ms: i64,
    pub(crate) source_sequence: u64,
    pub(crate) event_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RebuiltActivityPosition {
    pub(crate) sequence: u64,
    pub(crate) source_order: ActivityReadOrderKey,
}

pub(crate) fn map_rebuilt_read_sequence(
    prior_read_through: Option<&ActivityReadOrderKey>,
    rebuilt: &[RebuiltActivityPosition],
) -> u64 {
    let Some(read_through) = prior_read_through else {
        return 0;
    };
    rebuilt
        .iter()
        .filter(|position| &position.source_order <= read_through)
        .map(|position| position.sequence)
        .max()
        .unwrap_or(0)
}
