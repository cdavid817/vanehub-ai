use super::document_snapshot::DocumentAdmission;
use super::position_conversion::PositionConverter;
use crate::contexts::code_intelligence::domain::models::{
    CallDirection, NormalizedCallRelation, NormalizedHover, NormalizedLocation, NormalizedRange,
    NormalizedSymbol, PositionEncoding, QueryStatus,
};
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, DocumentSymbol,
    DocumentSymbolResponse, GotoDefinitionResponse, Hover, HoverContents, Location, LocationLink,
    MarkedString, OneOf, Range, SymbolKind, Uri, WorkspaceSymbolResponse,
};
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

pub(crate) const MAX_DEFINITIONS: usize = 20;
pub(crate) const MAX_REFERENCES: usize = 50;
pub(crate) const MAX_PREVIEW_BYTES: usize = 512;
pub(crate) const MAX_HOVER_SIGNATURE_BYTES: usize = 1_024;
pub(crate) const MAX_HOVER_DOCUMENTATION_BYTES: usize = 4_096;
pub(crate) const MAX_WORKSPACE_SYMBOLS: usize = 50;
pub(crate) const MAX_DOCUMENT_SYMBOLS: usize = 200;
pub(crate) const MAX_SYMBOL_NAME_BYTES: usize = 256;
/// How deep a nested document-symbol response is walked. The response is server-controlled, so an
/// unbounded walk is a stack overflow waiting for a malformed one.
pub(crate) const MAX_SYMBOL_DEPTH: usize = 8;
pub(crate) const MAX_CALL_RELATIONS: usize = 50;
pub(crate) const MAX_CALL_SITES: usize = 20;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticResultError {
    #[error("workspace is unavailable")]
    WorkspaceUnavailable,
}

/// The shape every bounded, workspace-filtered answer has: what survived, how much survived
/// before the cap, whether more existed, and how much the workspace check dropped. Three copies of
/// it were three chances for one of them to account differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedBatch<T> {
    pub(crate) items: Vec<T>,
    pub(crate) total: usize,
    pub(crate) truncated: bool,
    pub(crate) filtered_count: usize,
}

impl<T> NormalizedBatch<T> {
    /// `received` is what the server sent, so anything the normalization dropped is reported as
    /// filtered rather than going missing. `beyond` is what a bound left unexamined.
    fn bounded(mut items: Vec<T>, received: usize, beyond: usize, limit: usize) -> Self {
        let filtered_count = received.saturating_sub(items.len());
        let total = items.len();
        items.truncate(limit);
        Self {
            items,
            total,
            truncated: total > limit || beyond > 0,
            filtered_count,
        }
    }
}

pub(crate) type NormalizedLocations = NormalizedBatch<NormalizedLocation>;
pub(crate) type NormalizedSymbols = NormalizedBatch<NormalizedSymbol>;
pub(crate) type NormalizedCallRelations = NormalizedBatch<NormalizedCallRelation>;

/// What a nested walk accumulates. Separate from `NormalizedSymbols` because `unwalked` is a
/// property of the walk rather than of the answer.
#[derive(Default)]
struct SymbolWalk {
    symbols: Vec<NormalizedSymbol>,
    visited: usize,
    unwalked: usize,
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

    pub(crate) fn workspace_symbols(
        &self,
        response: Option<WorkspaceSymbolResponse>,
    ) -> NormalizedSymbols {
        let (visited, symbols) = match response {
            None => (0, Vec::new()),
            Some(WorkspaceSymbolResponse::Flat(entries)) => (
                entries.len(),
                entries
                    .into_iter()
                    .filter_map(|entry| self.flat_symbol(entry))
                    .collect(),
            ),
            Some(WorkspaceSymbolResponse::Nested(entries)) => (
                entries.len(),
                entries
                    .into_iter()
                    .filter_map(|entry| {
                        // A location without a range needs a resolve round trip this client does
                        // not make, so the entry is dropped rather than reported at a range we
                        // invented for it.
                        let OneOf::Left(location) = entry.location else {
                            return None;
                        };
                        self.symbol(
                            entry.name,
                            entry.kind,
                            entry.container_name,
                            self.normalize_location(location.uri, location.range),
                        )
                    })
                    .collect(),
            ),
        };
        NormalizedSymbols::bounded(symbols, visited, 0, MAX_WORKSPACE_SYMBOLS)
    }

    pub(crate) fn document_symbols(
        &self,
        relative_path: &str,
        response: Option<DocumentSymbolResponse>,
    ) -> NormalizedSymbols {
        let walk = match response {
            None => SymbolWalk::default(),
            // The flat form carries its own URIs, so it goes through the same workspace check the
            // location paths use rather than trusting the document it was requested for.
            Some(DocumentSymbolResponse::Flat(entries)) => SymbolWalk {
                visited: entries.len(),
                symbols: entries
                    .into_iter()
                    .filter_map(|entry| self.flat_symbol(entry))
                    .collect(),
                unwalked: 0,
            },
            Some(DocumentSymbolResponse::Nested(entries)) => {
                let mut walk = SymbolWalk::default();
                self.flatten(relative_path, &entries, None, 1, &mut walk);
                walk
            }
        };
        NormalizedSymbols::bounded(
            walk.symbols,
            walk.visited,
            walk.unwalked,
            MAX_DOCUMENT_SYMBOLS,
        )
    }

    /// Depth-first so a flattened list reads in source order, with each entry naming the symbol
    /// that encloses it -- the hierarchy the flattening throws away is otherwise unrecoverable.
    fn flatten(
        &self,
        relative_path: &str,
        entries: &[DocumentSymbol],
        container: Option<&str>,
        depth: usize,
        walk: &mut SymbolWalk,
    ) {
        for entry in entries {
            if depth > MAX_SYMBOL_DEPTH {
                walk.unwalked += 1;
                continue;
            }
            walk.visited += 1;
            if let Some(symbol) = self.symbol(
                entry.name.clone(),
                entry.kind,
                container.map(str::to_owned),
                self.normalize_in_document(relative_path, entry.selection_range),
            ) {
                walk.symbols.push(symbol);
            }
            if let Some(children) = entry.children.as_ref() {
                self.flatten(relative_path, children, Some(&entry.name), depth + 1, walk);
            }
        }
    }

    fn flat_symbol(&self, entry: lsp_types::SymbolInformation) -> Option<NormalizedSymbol> {
        self.symbol(
            entry.name,
            entry.kind,
            entry.container_name,
            self.normalize_location(entry.location.uri, entry.location.range),
        )
    }

    fn symbol(
        &self,
        name: String,
        kind: SymbolKind,
        container: Option<String>,
        location: Option<NormalizedLocation>,
    ) -> Option<NormalizedSymbol> {
        let (name, _) = truncate_utf8(&name, MAX_SYMBOL_NAME_BYTES);
        let (container, _) = bounded_optional(container, MAX_SYMBOL_NAME_BYTES);
        NormalizedSymbol::new(name, symbol_kind_id(kind), container, location?).ok()
    }

    pub(crate) fn call_relations(
        &self,
        direction: CallDirection,
        incoming: Option<Vec<CallHierarchyIncomingCall>>,
        outgoing: Option<Vec<CallHierarchyOutgoingCall>>,
    ) -> NormalizedCallRelations {
        let pairs: Vec<(CallHierarchyItem, Vec<Range>)> = match direction {
            CallDirection::Incoming => incoming
                .unwrap_or_default()
                .into_iter()
                .map(|call| (call.from, call.from_ranges))
                .collect(),
            CallDirection::Outgoing => outgoing
                .unwrap_or_default()
                .into_iter()
                .map(|call| (call.to, call.from_ranges))
                .collect(),
        };
        let received = pairs.len();
        let relations = pairs
            .into_iter()
            .filter_map(|(item, ranges)| self.call_relation(item, ranges))
            .collect();
        NormalizedCallRelations::bounded(relations, received, 0, MAX_CALL_RELATIONS)
    }

    fn call_relation(
        &self,
        item: CallHierarchyItem,
        ranges: Vec<Range>,
    ) -> Option<NormalizedCallRelation> {
        let location = self.normalize_location(item.uri, item.selection_range)?;
        // The sites are ranges in whichever file the direction implies, and the converter needs
        // that file's text. Only the ones in the item's own file can be converted, which is every
        // one of them for an incoming call and none of them for an outgoing call -- the protocol
        // gives outgoing sites relative to the caller, which is the document already open.
        let snapshot = self.admission.read(location.file()).ok();
        let call_sites = snapshot
            .iter()
            .flat_map(|snapshot| {
                let converter = PositionConverter::new(snapshot.text(), self.encoding);
                ranges
                    .iter()
                    .filter_map(move |range| converter.range_to_normalized(*range).ok())
            })
            .take(MAX_CALL_SITES)
            .collect::<Vec<NormalizedRange>>();
        let symbol = self.symbol(item.name, item.kind, item.detail, Some(location))?;
        Some(NormalizedCallRelation { symbol, call_sites })
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
        if sort {
            locations.sort_by(|left, right| location_key(left).cmp(&location_key(right)));
        }
        NormalizedLocations::bounded(locations, received, 0, limit)
    }

    fn normalize_location(&self, uri: Uri, range: Range) -> Option<NormalizedLocation> {
        let uri = Url::parse(uri.as_str()).ok()?;
        let canonical = uri.to_file_path().ok()?.canonicalize().ok()?;
        let relative = canonical.strip_prefix(&self.workspace_root).ok()?;
        self.normalize_in_document(&relative.to_string_lossy().replace('\\', "/"), range)
    }

    /// The half of `normalize_location` that runs once the file is known to be workspace-relative.
    /// Document symbols arrive without a URI, so they start here.
    fn normalize_in_document(&self, relative: &str, range: Range) -> Option<NormalizedLocation> {
        let snapshot = self.admission.read(relative).ok()?;
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

/// The protocol's numeric kinds, mapped once. An id the Agent reads has to be stable across server
/// versions, and a number is not something a tool result should make a reader look up.
const fn symbol_kind_id(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::FILE => "file",
        SymbolKind::MODULE => "module",
        SymbolKind::NAMESPACE => "namespace",
        SymbolKind::PACKAGE => "package",
        SymbolKind::CLASS => "class",
        SymbolKind::METHOD => "method",
        SymbolKind::PROPERTY => "property",
        SymbolKind::FIELD => "field",
        SymbolKind::CONSTRUCTOR => "constructor",
        SymbolKind::ENUM => "enum",
        SymbolKind::INTERFACE => "interface",
        SymbolKind::FUNCTION => "function",
        SymbolKind::VARIABLE => "variable",
        SymbolKind::CONSTANT => "constant",
        SymbolKind::STRING => "string",
        SymbolKind::NUMBER => "number",
        SymbolKind::BOOLEAN => "boolean",
        SymbolKind::ARRAY => "array",
        SymbolKind::OBJECT => "object",
        SymbolKind::KEY => "key",
        SymbolKind::NULL => "null",
        SymbolKind::ENUM_MEMBER => "enum_member",
        SymbolKind::STRUCT => "struct",
        SymbolKind::EVENT => "event",
        SymbolKind::OPERATOR => "operator",
        SymbolKind::TYPE_PARAMETER => "type_parameter",
        // The protocol reserves room to grow and a server may already use it. Reporting the
        // symbol under a placeholder beats dropping it for a kind nobody asked about.
        _ => "unknown",
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
