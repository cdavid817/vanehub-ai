use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::operation_service;

pub(crate) fn assemble_operations_api() -> OperationsApi {
    OperationsApi::new(operation_service())
}
