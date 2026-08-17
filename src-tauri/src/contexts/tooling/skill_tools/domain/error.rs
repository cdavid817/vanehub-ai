/// Every way a Skill tool manifest, identity, limit, or schema can fail its contract.
///
/// Carries a stable `code()` because these reach unified logs and, later, the governance UI:
/// a machine-readable reason survives message wording changes and stays safe to persist, while
/// the free-form detail stays bounded and never contains file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolDomainError {
    InvalidToolId(String),
    InvalidOwnerId(String),
    WorkspacePathRequired,
    InvalidRevisionWitness,
    CanonicalNameTooLong { maximum: usize, actual: usize },
    ManifestNotAnObject,
    UnsupportedManifestVersion { supported: u32, actual: u64 },
    MissingField(&'static str),
    UnexpectedFieldType(&'static str),
    UnknownField(String),
    TooLarge { maximum: u64, actual: u64 },
    EmptyToolList,
    TooManyTools { maximum: usize, actual: usize },
    DuplicateToolId(String),
    DescriptionTooLong { maximum: usize, actual: usize },
    ForbiddenImplementationKind(String),
    UnknownImplementationKind(String),
    UnsafeImplementationPath(String),
    InvalidContentHash(String),
    UnknownCapability(String),
    DuplicateCapability(String),
    TooManyCapabilities { maximum: usize, actual: usize },
    UndeclaredCapability(String),
    InvalidTemplate(String),
    TooManyTemplateFields { maximum: usize, actual: usize },
    InvalidPermissionManifest(String),
    LimitAboveCeiling { limit: &'static str },
    Schema(BoundedSchemaError),
}

impl SkillToolDomainError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidToolId(_) => "invalid-tool-id",
            Self::InvalidOwnerId(_) => "invalid-skill-id",
            Self::WorkspacePathRequired => "workspace-path-required",
            Self::InvalidRevisionWitness => "invalid-revision-witness",
            Self::CanonicalNameTooLong { .. } => "canonical-name-too-long",
            Self::ManifestNotAnObject => "manifest-not-an-object",
            Self::UnsupportedManifestVersion { .. } => "unsupported-manifest-version",
            Self::MissingField(_) => "missing-field",
            Self::UnexpectedFieldType(_) => "unexpected-field-type",
            Self::UnknownField(_) => "unknown-field",
            Self::TooLarge { .. } => "manifest-too-large",
            Self::EmptyToolList => "empty-tool-list",
            Self::TooManyTools { .. } => "too-many-tools",
            Self::DuplicateToolId(_) => "duplicate-tool-id",
            Self::DescriptionTooLong { .. } => "description-too-long",
            Self::ForbiddenImplementationKind(_) => "forbidden-implementation-kind",
            Self::UnknownImplementationKind(_) => "unknown-implementation-kind",
            Self::UnsafeImplementationPath(_) => "unsafe-implementation-path",
            Self::InvalidContentHash(_) => "invalid-content-hash",
            Self::UnknownCapability(_) => "unknown-capability",
            Self::DuplicateCapability(_) => "duplicate-capability",
            Self::TooManyCapabilities { .. } => "too-many-capabilities",
            Self::UndeclaredCapability(_) => "undeclared-capability",
            Self::InvalidTemplate(_) => "invalid-template",
            Self::TooManyTemplateFields { .. } => "too-many-template-fields",
            Self::InvalidPermissionManifest(_) => "invalid-permission-manifest",
            Self::LimitAboveCeiling { .. } => "limit-above-ceiling",
            Self::Schema(error) => error.code(),
        }
    }
}

impl From<BoundedSchemaError> for SkillToolDomainError {
    fn from(error: BoundedSchemaError) -> Self {
        Self::Schema(error)
    }
}

/// Why a declared input/output schema falls outside the supported JSON Schema subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BoundedSchemaError {
    NotAnObject,
    RootMustBeObjectType,
    MissingType,
    UnsupportedType(String),
    UnsupportedKeyword(String),
    UnsupportedCombinator(String),
    TooDeep { maximum: usize, actual: usize },
    TooManyNodes { maximum: usize, actual: usize },
    TooManyProperties { maximum: usize, actual: usize },
    TooManyEnumValues { maximum: usize, actual: usize },
    TooLarge { maximum: usize, actual: usize },
    MissingItems,
    RequiredMustBeStrings,
    RequiredPropertyNotDeclared(String),
}

impl BoundedSchemaError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::NotAnObject => "schema-not-an-object",
            Self::RootMustBeObjectType => "schema-root-must-be-object",
            Self::MissingType => "schema-missing-type",
            Self::UnsupportedType(_) => "schema-unsupported-type",
            Self::UnsupportedKeyword(_) => "schema-unsupported-keyword",
            Self::UnsupportedCombinator(_) => "schema-unsupported-combinator",
            Self::TooDeep { .. } => "schema-too-deep",
            Self::TooManyNodes { .. } => "schema-too-many-nodes",
            Self::TooManyProperties { .. } => "schema-too-many-properties",
            Self::TooManyEnumValues { .. } => "schema-too-many-enum-values",
            Self::TooLarge { .. } => "schema-too-large",
            Self::MissingItems => "schema-missing-items",
            Self::RequiredMustBeStrings => "schema-required-must-be-strings",
            Self::RequiredPropertyNotDeclared(_) => "schema-required-property-not-declared",
        }
    }
}
