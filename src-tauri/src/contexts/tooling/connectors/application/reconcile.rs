// The report that surfaces this lands with the Connector Lifecycle task group.
#![cfg_attr(not(test), allow(dead_code))]

//! Reconciling connector subjects against the snapshot the platform is actually running.
//!
//! ## The read order is the consistency argument
//!
//! The platform is asked **first**, and everything else is keyed on the snapshot it named. That
//! ordering is what makes the verdict internally consistent without a snapshot shared across two
//! subdomains — which is not available, because sharing one would mean handing a live
//! `rusqlite::Transaction` across a published context API and holding a read snapshot open across
//! another context's work.
//!
//! It is sound because definition revisions are immutable and keyed by snapshot. If an activation
//! commits between the two reads, the revision looked up is still the one belonging to the
//! snapshot that was named, so the verdict describes one whole generation — at worst one
//! activation stale, never a mixture. Reading the revision first and the platform second is what
//! would produce a mixture, which is why the order is stated here rather than left to whoever
//! edits this next.
//!
//! Nothing here writes. The repositories are used through their reading methods only, and the
//! tests drive fakes that panic on every write method so that stays true.

use super::{
    ActiveConnectorSnapshotPort, ConnectorDefinitionRepository, ConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::domain::{
    judge_connector, ActiveConnectorSnapshot, ConnectorDefinitionRevision, ConnectorFacts,
    ConnectorGlobalId, ConnectorVerdict,
};

/// Every subject, and what this installation currently knows about it.
///
/// Ordered by connector id so two runs against the same database produce the same report.
pub(crate) fn reconcile_connectors(
    subjects: &dyn ConnectorSubjectRepository,
    definitions: &dyn ConnectorDefinitionRepository,
    active: &dyn ActiveConnectorSnapshotPort,
) -> Result<Vec<ConnectorVerdict>, String> {
    let mut verdicts = Vec::new();
    for subject in subjects.all()? {
        verdicts.push(reconcile_connector(
            &subject.connector,
            definitions,
            active,
        )?);
    }
    Ok(verdicts)
}

/// One subject's readiness, against the snapshot the platform is running.
pub(crate) fn reconcile_connector(
    connector: &ConnectorGlobalId,
    definitions: &dyn ConnectorDefinitionRepository,
    active: &dyn ActiveConnectorSnapshotPort,
) -> Result<ConnectorVerdict, String> {
    // The platform first. See the module header.
    let active = active.active_snapshot(connector)?;

    // Looked up only for a running snapshot, and only for that snapshot. Reaching for any other
    // revision is the defect this ordering exists to prevent.
    let recorded_at_active = match &active {
        ActiveConnectorSnapshot::Running { snapshot, .. } => definitions
            .recorded(connector, snapshot)?
            .map(|revision| revision.digest),
        _ => None,
    };

    // Only distinguishes "uninstalled, with evidence of what it was" from "a subject that exists
    // because an instance mentions it". Never used to pick a revision.
    let has_any_revision = !definitions.revisions(connector)?.is_empty();

    Ok(judge_connector(&ConnectorFacts {
        connector: connector.clone(),
        active,
        recorded_at_active,
        has_any_revision,
    }))
}

/// Every revision recorded for a subject, for a diagnostic listing.
///
/// **Never a readiness input.** Recording order leads with a version that was recorded but never
/// activated, and after a rollback it still leads with the abandoned newer one.
pub(crate) fn recorded_revisions(
    connector: &ConnectorGlobalId,
    definitions: &dyn ConnectorDefinitionRepository,
) -> Result<Vec<ConnectorDefinitionRevision>, String> {
    definitions.revisions(connector)
}
