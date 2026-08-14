use super::{DelegationApplyPlan, DelegationChangeFile, DelegationChangeKind};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationApplyPathExpectation {
    pub(crate) path: String,
    pub(crate) must_exist: bool,
    pub(crate) expected_mode: Option<String>,
    pub(crate) expected_git_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationApplyPathWitness {
    pub(crate) path: String,
    pub(crate) exists: bool,
    pub(crate) mode: Option<String>,
    pub(crate) git_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationRecoveryCapsule {
    pub(crate) apply_attempt_id: String,
    pub(crate) reference: String,
    pub(crate) witness_hash: String,
}

pub(crate) trait DelegationApplyStagingPort: Send + Sync {
    fn inspect_paths(
        &self,
        plan: &DelegationApplyPlan,
        expectations: &[DelegationApplyPathExpectation],
    ) -> Result<Vec<DelegationApplyPathWitness>, ()>;

    fn stage_recovery_capsule(
        &self,
        plan: &DelegationApplyPlan,
        apply_attempt_id: &str,
        expectations: &[DelegationApplyPathExpectation],
    ) -> Result<DelegationRecoveryCapsule, ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationApplyStagingError {
    InvalidRequest,
    InspectionFailure,
    ConcurrentMutation,
    CapsuleFailure,
    InvalidCapsule,
}

pub(crate) struct DelegationApplyStagingService {
    port: Arc<dyn DelegationApplyStagingPort>,
}

impl DelegationApplyStagingService {
    pub(crate) fn new(port: Arc<dyn DelegationApplyStagingPort>) -> Self {
        Self { port }
    }

    pub(crate) fn stage(
        &self,
        plan: &DelegationApplyPlan,
        apply_attempt_id: &str,
    ) -> Result<DelegationRecoveryCapsule, DelegationApplyStagingError> {
        if apply_attempt_id.trim().is_empty() {
            return Err(DelegationApplyStagingError::InvalidRequest);
        }
        let expectations = expectations(&plan.artifact.capture.files)?;
        let witnesses = self
            .port
            .inspect_paths(plan, &expectations)
            .map_err(|_| DelegationApplyStagingError::InspectionFailure)?;
        verify_witnesses(&expectations, &witnesses)?;
        let capsule = self
            .port
            .stage_recovery_capsule(plan, apply_attempt_id, &expectations)
            .map_err(|_| DelegationApplyStagingError::CapsuleFailure)?;
        if capsule.apply_attempt_id != apply_attempt_id
            || capsule.reference.trim().is_empty()
            || !valid_sha256(&capsule.witness_hash)
        {
            return Err(DelegationApplyStagingError::InvalidCapsule);
        }
        Ok(capsule)
    }
}

fn expectations(
    files: &[DelegationChangeFile],
) -> Result<Vec<DelegationApplyPathExpectation>, DelegationApplyStagingError> {
    let mut output = Vec::new();
    let mut paths = BTreeSet::new();
    for file in files {
        match file.kind {
            DelegationChangeKind::Added => push_absent(&mut output, &mut paths, &file.path)?,
            DelegationChangeKind::Modified
            | DelegationChangeKind::Deleted
            | DelegationChangeKind::TypeChanged => {
                push_existing(&mut output, &mut paths, &file.path, file)?
            }
            DelegationChangeKind::Renamed => {
                let previous = file
                    .previous_path
                    .as_deref()
                    .ok_or(DelegationApplyStagingError::InvalidRequest)?;
                push_existing(&mut output, &mut paths, previous, file)?;
                push_absent(&mut output, &mut paths, &file.path)?;
            }
        }
        if file.kind != DelegationChangeKind::Deleted
            && (file.after_mode.is_none() || file.after_git_hash.is_none())
        {
            return Err(DelegationApplyStagingError::InvalidRequest);
        }
    }
    Ok(output)
}

fn push_existing(
    output: &mut Vec<DelegationApplyPathExpectation>,
    paths: &mut BTreeSet<String>,
    path: &str,
    file: &DelegationChangeFile,
) -> Result<(), DelegationApplyStagingError> {
    if !paths.insert(path.to_owned())
        || file.before_mode.is_none()
        || file.before_git_hash.is_none()
    {
        return Err(DelegationApplyStagingError::InvalidRequest);
    }
    output.push(DelegationApplyPathExpectation {
        path: path.to_owned(),
        must_exist: true,
        expected_mode: file.before_mode.clone(),
        expected_git_hash: file.before_git_hash.clone(),
    });
    Ok(())
}

fn push_absent(
    output: &mut Vec<DelegationApplyPathExpectation>,
    paths: &mut BTreeSet<String>,
    path: &str,
) -> Result<(), DelegationApplyStagingError> {
    if !paths.insert(path.to_owned()) {
        return Err(DelegationApplyStagingError::InvalidRequest);
    }
    output.push(DelegationApplyPathExpectation {
        path: path.to_owned(),
        must_exist: false,
        expected_mode: None,
        expected_git_hash: None,
    });
    Ok(())
}

fn verify_witnesses(
    expected: &[DelegationApplyPathExpectation],
    actual: &[DelegationApplyPathWitness],
) -> Result<(), DelegationApplyStagingError> {
    let actual = actual
        .iter()
        .map(|witness| (witness.path.as_str(), witness))
        .collect::<BTreeMap<_, _>>();
    if actual.len() != expected.len() {
        return Err(DelegationApplyStagingError::ConcurrentMutation);
    }
    for expectation in expected {
        let witness = actual
            .get(expectation.path.as_str())
            .ok_or(DelegationApplyStagingError::ConcurrentMutation)?;
        if witness.exists != expectation.must_exist
            || witness.mode != expectation.expected_mode
            || witness.git_hash != expectation.expected_git_hash
        {
            return Err(DelegationApplyStagingError::ConcurrentMutation);
        }
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "apply_staging_tests.rs"]
mod tests;
