use super::document_snapshot::DocumentAdmission;
use super::position_conversion::PositionConverter;
use crate::contexts::code_intelligence::domain::models::{
    NormalizedHover, NormalizedLocation, PositionEncoding, QueryStatus,
};
use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, Location, LocationLink, MarkedString, Range, Uri,
};
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

pub(crate) const MAX_DEFINITIONS: usize = 20;
pub(crate) const MAX_REFERENCES: usize = 50;
pub(crate) const MAX_PREVIEW_BYTES: usize = 512;
pub(crate) const MAX_HOVER_SIGNATURE_BYTES: usize = 1_024;
pub(crate) const MAX_HOVER_DOCUMENTATION_BYTES: usize = 4_096;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticResultError {
    #[error("workspace is unavailable")]
    WorkspaceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedLocations {
    pub(crate) locations: Vec<NormalizedLocation>,
    pub(crate) total: usize,
    pub(crate) truncated: bool,
    pub(crate) filtered_count: usize,
}

pub(crate) struct SemanticResultNormalizer {
    workspace_root: PathBuf,
    admission: DocumentAdmission,
    encoding: PositionEncoding,
}

impl SemanticResultNormalizer {
    pub(crate) fn new(
        workspace_root: &Path,
        encoding: PositionEncoding,
    ) -> Result<Self, SemanticResultError> {
        let workspace_root = workspace_root
            .canonicalize()
            .map_err(|_| SemanticResultError::WorkspaceUnavailable)?;
        let admission = DocumentAdmission::new(&workspace_root)
            .map_err(|_| SemanticResultError::WorkspaceUnavailable)?;
        Ok(Self {
            workspace_root,
            admission,
            encoding,
        })
    }

    pub(crate) fn definitions(
        &self,
        response: Option<GotoDefinitionResponse>,
    ) -> NormalizedLocations {
        let targets = match response {
            None => Vec::new(),
            Some(GotoDefinitionResponse::Scalar(location)) => vec![location_target(location)],
            Some(GotoDefinitionResponse::Array(locations)) => {
                locations.into_iter().map(location_target).collect()
            }
            Some(GotoDefinitionResponse::Link(links)) => {
                links.into_iter().map(link_target).collect()
            }
        };
        self.normalize_locations(targets, MAX_DEFINITIONS, false)
    }

    pub(crate) fn references(&self, locations: Vec<Location>) -> NormalizedLocations {
        self.normalize_locations(
            locations.into_iter().map(location_target).collect(),
            MAX_REFERENCES,
            true,
        )
    }

    pub(crate) fn hover(
        &self,
        document_text: &str,
        hover: Option<Hover>,
    ) -> Option<NormalizedHover> {
        let hover = hover?;
        let (signature, documentation) = normalize_hover_contents(hover.contents);
        let range = hover.range.and_then(|range| {
            PositionConverter::new(document_text, self.encoding)
                .range_to_normalized(range)
                .ok()
        });
        let (signature, signature_truncated) =
            bounded_optional(signature, MAX_HOVER_SIGNATURE_BYTES);
        let (documentation, documentation_truncated) =
            bounded_optional(documentation, MAX_HOVER_DOCUMENTATION_BYTES);
        Some(NormalizedHover {
            signature,
            documentation,
            range,
            truncated: signature_truncated || documentation_truncated,
        })
    }

    fn normalize_locations(
        &self,
        targets: Vec<(Uri, Range)>,
        limit: usize,
        sort: bool,
    ) -> NormalizedLocations {
        let received = targets.len();
        let mut locations = targets
            .into_iter()
            .filter_map(|(uri, range)| self.normalize_location(uri, range))
            .collect::<Vec<_>>();
        let filtered_count = received.saturating_sub(locations.len());
        if sort {
            locations.sort_by(|left, right| location_key(left).cmp(&location_key(right)));
        }
        let total = locations.len();
        locations.truncate(limit);
        NormalizedLocations {
            locations,
            total,
            truncated: total > limit,
            filtered_count,
        }
    }

    fn normalize_location(&self, uri: Uri, range: Range) -> Option<NormalizedLocation> {
        let uri = Url::parse(uri.as_str()).ok()?;
        let canonical = uri.to_file_path().ok()?.canonicalize().ok()?;
        let relative = canonical.strip_prefix(&self.workspace_root).ok()?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        let snapshot = self.admission.read(&relative).ok()?;
        let normalized_range = PositionConverter::new(snapshot.text(), self.encoding)
            .range_to_normalized(range)
            .ok()?;
        let preview = preview_line(snapshot.text(), normalized_range.start_line);
        NormalizedLocation::new(relative, normalized_range, preview).ok()
    }
}

pub(crate) const fn query_status_label(status: QueryStatus) -> &'static str {
    match status {
        QueryStatus::Ready => "ready",
        QueryStatus::Warming => "warming",
        QueryStatus::Timeout => "timeout",
        QueryStatus::Unavailable => "unavailable",
        QueryStatus::Failed => "failed",
    }
}

fn location_target(location: Location) -> (Uri, Range) {
    (location.uri, location.range)
}

fn link_target(link: LocationLink) -> (Uri, Range) {
    (link.target_uri, link.target_selection_range)
}

fn location_key(location: &NormalizedLocation) -> (&str, u32, u32, u32, u32) {
    (
        location.file(),
        location.range.start_line,
        location.range.start_column,
        location.range.end_line,
        location.range.end_column,
    )
}

fn preview_line(text: &str, line: u32) -> Option<String> {
    let index = usize::try_from(line.checked_sub(1)?).ok()?;
    let value = text.lines().nth(index)?.trim();
    (!value.is_empty()).then(|| truncate_utf8(value, MAX_PREVIEW_BYTES).0)
}

fn normalize_hover_contents(contents: HoverContents) -> (Option<String>, Option<String>) {
    let mut signatures = Vec::new();
    let mut documentation = Vec::new();
    let values = match contents {
        HoverContents::Scalar(value) => vec![value],
        HoverContents::Array(values) => values,
        HoverContents::Markup(markup) => {
            return (None, non_empty(strip_executable_html(&markup.value)));
        }
    };
    for value in values {
        match value {
            MarkedString::LanguageString(value) => signatures.push(value.value),
            MarkedString::String(value) => documentation.push(strip_executable_html(&value)),
        }
    }
    (
        non_empty(signatures.join("\n")),
        non_empty(documentation.join("\n\n")),
    )
}

fn strip_executable_html(value: &str) -> String {
    value
        .replace("<script", "&lt;script")
        .replace("</script", "&lt;/script")
        .replace("<iframe", "&lt;iframe")
        .replace("</iframe", "&lt;/iframe")
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn bounded_optional(value: Option<String>, limit: usize) -> (Option<String>, bool) {
    match value {
        Some(value) => {
            let (value, truncated) = truncate_utf8(&value, limit);
            (Some(value), truncated)
        }
        None => (None, false),
    }
}

fn truncate_utf8(value: &str, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value.to_owned(), false);
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_owned(), true)
}
