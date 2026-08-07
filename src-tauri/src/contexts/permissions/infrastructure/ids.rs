use crate::contexts::permissions::application::PermissionsIdPort;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PermissionsUuidIdGenerator;

impl PermissionsIdPort for PermissionsUuidIdGenerator {
    fn next_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}", uuid::Uuid::new_v4())
    }
}
