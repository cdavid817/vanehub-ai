use super::ExecutionDomainError;

const MAX_PAGE_SIZE: u16 = 100;
const MAX_PAGE_TOKEN_LENGTH: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageRequest {
    pub(crate) limit: u16,
    pub(crate) page_token: Option<String>,
}

impl PageRequest {
    pub(crate) fn new(
        limit: u16,
        page_token: Option<String>,
    ) -> Result<Self, ExecutionDomainError> {
        if limit == 0 || limit > MAX_PAGE_SIZE {
            return Err(ExecutionDomainError::InvalidPageSize {
                max: MAX_PAGE_SIZE as usize,
            });
        }
        if page_token
            .as_ref()
            .is_some_and(|token| token.chars().count() > MAX_PAGE_TOKEN_LENGTH)
        {
            return Err(ExecutionDomainError::InvalidPageToken {
                max: MAX_PAGE_TOKEN_LENGTH,
            });
        }
        Ok(Self { limit, page_token })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) next_page_token: Option<String>,
}
