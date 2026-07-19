use super::{CategoryId, SessionsDomainError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryName(String);

impl CategoryName {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SessionsDomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(SessionsDomainError::CategoryNameRequired)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionCategory {
    id: CategoryId,
    name: CategoryName,
    sort_order: i64,
}

impl SessionCategory {
    pub(crate) fn new(id: CategoryId, name: CategoryName, sort_order: i64) -> Self {
        Self {
            id,
            name,
            sort_order,
        }
    }

    pub(crate) fn rename(&mut self, name: CategoryName) {
        self.name = name;
    }

    pub(crate) fn id(&self) -> &CategoryId {
        &self.id
    }

    pub(crate) fn name(&self) -> &CategoryName {
        &self.name
    }

    pub(crate) fn sort_order(&self) -> i64 {
        self.sort_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_names_are_trimmed_and_empty_names_are_rejected() {
        let id = CategoryId::parse("category-1").expect("category id");
        let mut category = SessionCategory::new(
            id,
            CategoryName::parse("  Work  ").expect("category name"),
            3,
        );
        category.rename(CategoryName::parse("Review").expect("renamed category"));

        assert_eq!(category.id().as_str(), "category-1");
        assert_eq!(category.name().as_str(), "Review");
        assert_eq!(category.sort_order(), 3);
        assert_eq!(
            CategoryName::parse(" \t "),
            Err(SessionsDomainError::CategoryNameRequired)
        );
    }
}
