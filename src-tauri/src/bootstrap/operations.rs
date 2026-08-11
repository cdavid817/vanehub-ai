use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::persistent_operation_service;
use crate::platform::database::NativeDatabase;

pub(crate) fn assemble_operations_api(database: NativeDatabase) -> OperationsApi {
    OperationsApi::new(persistent_operation_service(database))
}
