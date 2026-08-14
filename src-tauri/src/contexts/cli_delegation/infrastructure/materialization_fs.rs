use crate::contexts::cli_delegation::application::{
    DelegationMaterializationError, DelegationMaterializationPort,
};
use crate::platform::filesystem::create_new_file;
use std::io::Write;
use std::path::Path;

pub(crate) struct SystemDelegationMaterializationAdapter;

impl DelegationMaterializationPort for SystemDelegationMaterializationAdapter {
    fn write_new(
        &self,
        path: &Path,
        bytes: &[u8],
        readonly: bool,
    ) -> Result<(), DelegationMaterializationError> {
        let mut file =
            create_new_file(path).map_err(|_| DelegationMaterializationError::StorageFailure)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| DelegationMaterializationError::StorageFailure)?;
        if readonly {
            let mut permissions = file
                .metadata()
                .map_err(|_| DelegationMaterializationError::StorageFailure)?
                .permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions)
                .map_err(|_| DelegationMaterializationError::StorageFailure)?;
        }
        Ok(())
    }
}
