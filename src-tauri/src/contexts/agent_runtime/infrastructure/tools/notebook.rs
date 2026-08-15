//! Reads and edits a Jupyter notebook a cell at a time, so the model never handles notebook JSON.
//!
//! Two things make a notebook unworkable through the ordinary file tools, and both are why this
//! exists. Reading one through `file` spends the context window on the container rather than the
//! code: a four-cell notebook with one plot measured 122,675 characters on disk, of which 135 were
//! cell source, the rest being JSON scaffolding and one 120,039-character base64 PNG. And editing
//! one through `edit` is not merely awkward -- cell source is an array of escaped per-line strings,
//! so a two-line change spans two array elements and has no unique exact match to anchor on.
//!
//! Rewrites splice raw cell text rather than re-serializing the notebook. `serde_json` here sorts
//! object keys, so a parse-then-write would reorder every object in the file and turn a one-cell
//! edit into a whole-file diff (`add-agent-notebook-tool` D4).

use super::walk::{exceeds_size_limit, MAX_FILE_BYTES};
use super::{ToolExecutionOutcome, MAX_TOOL_OUTPUT_BYTES};
use crate::platform::filesystem::{BoundaryError, BoundedFilesystem};
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// Per-output text kept in a read. Enough for a repr line or the head of a traceback; past this the
/// output is a body of results the model should re-run rather than read out of the file.
const MAX_OUTPUT_TEXT_CHARS: usize = 600;

/// The only notebook format this understands. Refusing anything else is deliberate: partially
/// understanding a notebook and writing a guess back destroys work with no other copy (D6).
const SUPPORTED_NBFORMAT: u64 = 4;

/// A notebook decoded far enough to work with, keeping every cell's original bytes so an untouched
/// cell can be written back exactly as it was stored.
struct Notebook {
    /// Top-level members other than `cells`, each held as its original text.
    members: Vec<(String, Box<RawValue>)>,
    cells: Vec<Cell>,
    /// Whether cells carry `id`, which nbformat 4.5 requires and 4.0-4.4 do not have.
    uses_cell_ids: bool,
}

struct Cell {
    raw: Box<RawValue>,
    id: Option<String>,
    cell_type: String,
}

/// One notebook call. Grouped rather than passed as eight positional arguments, following
/// `GrepRequest`: most of these are only meaningful for some operations, and a struct says which
/// at the call site.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NotebookRequest<'a> {
    pub(crate) operation: &'a str,
    pub(crate) path: &'a str,
    pub(crate) cell_id: Option<&'a str>,
    pub(crate) cell_index: Option<usize>,
    pub(crate) source: Option<&'a str>,
    pub(crate) cell_type: Option<&'a str>,
    pub(crate) position: Option<&'a str>,
}

pub(crate) fn execute_notebook(
    request: NotebookRequest<'_>,
    workspace_folder: &str,
) -> ToolExecutionOutcome {
    let NotebookRequest {
        operation,
        path,
        cell_id,
        cell_index,
        source,
        cell_type,
        position,
    } = request;
    let boundary = match BoundedFilesystem::new(Path::new(workspace_folder)) {
        Ok(boundary) => boundary,
        Err(failure) => return error(&format!("Workspace folder is unavailable: {failure}")),
    };
    let resolved = match boundary.resolve_existing(path) {
        Ok(resolved) => resolved,
        Err(BoundaryError::Io(io_error)) if io_error.kind() == std::io::ErrorKind::NotFound => {
            return error(&format!(
                "\"{path}\" does not exist in this workspace. Verify the path, e.g. with the search tools, before retrying."
            ));
        }
        Err(failure) => return error(&format!("Path \"{path}\" is not accessible: {failure}")),
    };
    if exceeds_size_limit(&resolved) {
        return error(&format!(
            "\"{path}\" is larger than the {} MB limit.",
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let text = match std::fs::read_to_string(&resolved) {
        Ok(text) => text,
        Err(failure) => return error(&format!("Failed to read \"{path}\": {failure}")),
    };
    let mut notebook = match Notebook::parse(&text) {
        Ok(notebook) => notebook,
        Err(reason) => return error(&format!("\"{path}\" {reason}")),
    };

    match operation {
        "read" => read_notebook(&notebook, path),
        "replace" | "insert" | "delete" => {
            let outcome = match operation {
                "replace" => notebook.replace(cell_id, cell_index, source),
                "insert" => notebook.insert(cell_id, cell_index, source, cell_type, position),
                _ => notebook.delete(cell_id, cell_index),
            };
            match outcome {
                Ok(summary) => {
                    match super::edit_tool::write_atomically(&resolved, &notebook.render()) {
                        Ok(()) => ToolExecutionOutcome {
                            output: format!("{summary} in \"{path}\"."),
                            is_error: false,
                        },
                        Err(failure) => error(&format!("Failed to write \"{path}\": {failure}")),
                    }
                }
                Err(message) => error(&message),
            }
        }
        other => error(&format!("Unknown notebook operation \"{other}\".")),
    }
}

fn error(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_string(),
        is_error: true,
    }
}

impl Notebook {
    fn parse(text: &str) -> Result<Self, String> {
        // A plain BTreeMap, not serde_json::Map, which only holds Value. Sorted either way, which
        // is why top-level key order is the one thing render() has to reimpose.
        let root: BTreeMap<String, Box<RawValue>> = serde_json::from_str(text)
            .map_err(|_| "is not valid JSON and cannot be read as a notebook.".to_string())?;
        let format = root
            .get("nbformat")
            .and_then(|raw| serde_json::from_str::<u64>(raw.get()).ok());
        match format {
            Some(SUPPORTED_NBFORMAT) => {}
            Some(other) => {
                return Err(format!(
                    "declares nbformat {other}; only nbformat {SUPPORTED_NBFORMAT} is supported."
                ))
            }
            None => return Err("has no nbformat field and is not a notebook.".to_string()),
        }
        let raw_cells: Vec<Box<RawValue>> = root
            .get("cells")
            .ok_or_else(|| "has no cells array and is not a notebook.".to_string())
            .and_then(|raw| {
                serde_json::from_str(raw.get())
                    .map_err(|_| "has a cells field that is not an array.".to_string())
            })?;

        let mut cells = Vec::with_capacity(raw_cells.len());
        let mut uses_cell_ids = false;
        for raw in raw_cells {
            let decoded: Map<String, Value> = serde_json::from_str(raw.get())
                .map_err(|_| "contains a cell that is not an object.".to_string())?;
            let id = decoded.get("id").and_then(Value::as_str).map(str::to_owned);
            uses_cell_ids |= id.is_some();
            let cell_type = decoded
                .get("cell_type")
                .and_then(Value::as_str)
                .unwrap_or("code")
                .to_owned();
            cells.push(Cell { raw, id, cell_type });
        }
        let members = root.into_iter().filter(|(key, _)| key != "cells").collect();
        Ok(Self {
            members,
            cells,
            uses_cell_ids,
        })
    }
}

/// Joins nbformat's per-line source array back into text. Source may also be a plain string, which
/// the format allows and some writers produce.
fn source_text(cell: &Map<String, Value>) -> String {
    match cell.get("source") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(lines)) => lines
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .concat(),
        _ => String::new(),
    }
}

/// One line per output, naming what it is. Bytes never appear: an `image/png` output is the single
/// biggest thing in a notebook and the whole reason reading one is broken today (D2).
fn summarize_output(output: &Value) -> String {
    let Some(output) = output.as_object() else {
        return "  [output: unreadable]".to_string();
    };
    let kind = output
        .get("output_type")
        .and_then(Value::as_str)
        .unwrap_or("output");
    if kind == "error" {
        let name = output
            .get("ename")
            .and_then(Value::as_str)
            .unwrap_or("Error");
        let value = output.get("evalue").and_then(Value::as_str).unwrap_or("");
        return format!("  [error] {name}: {}", bounded(value));
    }
    if kind == "stream" {
        let text = match output.get("text") {
            Some(Value::String(text)) => text.clone(),
            Some(Value::Array(lines)) => lines
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .concat(),
            _ => String::new(),
        };
        return format!("  [stream] {}", bounded(&text));
    }
    let Some(data) = output.get("data").and_then(Value::as_object) else {
        return format!("  [{kind}]");
    };
    // Text first when present: it is the representation worth reading. Every other media type is
    // named and measured, never carried.
    let mut parts: Vec<String> = Vec::new();
    if let Some(text) = data.get("text/plain") {
        let text = match text {
            Value::String(text) => text.clone(),
            Value::Array(lines) => lines
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .concat(),
            _ => String::new(),
        };
        parts.push(bounded(&text));
    }
    for (media_type, value) in data {
        if media_type == "text/plain" {
            continue;
        }
        let bytes = match value {
            Value::String(encoded) => encoded.len(),
            Value::Array(lines) => lines.iter().filter_map(Value::as_str).map(str::len).sum(),
            other => other.to_string().len(),
        };
        parts.push(format!("<{media_type}, {bytes} bytes>"));
    }
    format!("  [{kind}] {}", parts.join(" "))
}

fn bounded(text: &str) -> String {
    let trimmed = text.trim_end();
    match trimmed.char_indices().nth(MAX_OUTPUT_TEXT_CHARS) {
        Some((byte_index, _)) => format!("{}… [truncated]", &trimmed[..byte_index]),
        None => trimmed.to_owned(),
    }
}

fn read_notebook(notebook: &Notebook, path: &str) -> ToolExecutionOutcome {
    let total = notebook.cells.len();
    let mut rendered = String::new();
    let mut shown = 0usize;
    for (index, cell) in notebook.cells.iter().enumerate() {
        let decoded: Map<String, Value> = match serde_json::from_str(cell.raw.get()) {
            Ok(decoded) => decoded,
            Err(_) => continue,
        };
        let id = cell.id.clone().unwrap_or_else(|| "-".to_owned());
        let mut entry = format!(
            "[{index}] id={id} {}\n{}\n",
            cell.cell_type,
            source_text(&decoded)
        );
        if let Some(outputs) = decoded.get("outputs").and_then(Value::as_array) {
            for output in outputs {
                entry.push_str(&summarize_output(output));
                entry.push('\n');
            }
        }
        // A truncated read must say so rather than look like a short notebook, so the budget is
        // checked before appending and the count reported below reflects what was actually shown.
        if rendered.len() + entry.len() > MAX_TOOL_OUTPUT_BYTES.saturating_sub(200) {
            break;
        }
        rendered.push_str(&entry);
        rendered.push('\n');
        shown += 1;
    }
    let header = if shown == total {
        format!("\"{path}\": {total} cells.\n\n")
    } else {
        format!("\"{path}\": showing {shown} of {total} cells; read again for the rest.\n\n")
    };
    ToolExecutionOutcome {
        output: format!("{header}{rendered}"),
        is_error: false,
    }
}

impl Notebook {
    /// Resolves exactly one addressing form to a position. Both or neither is a caller error rather
    /// than something to guess at, and a duplicated id is reported rather than silently taking the
    /// first (D3).
    fn locate(&self, cell_id: Option<&str>, cell_index: Option<usize>) -> Result<usize, String> {
        match (cell_id, cell_index) {
            (Some(_), Some(_)) => Err("Supply either cell_id or cell_index, not both.".to_string()),
            (None, None) => {
                Err("Supply cell_id or cell_index to say which cell to act on.".to_string())
            }
            (Some(id), None) => {
                let matches: Vec<usize> = self
                    .cells
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| cell.id.as_deref() == Some(id))
                    .map(|(index, _)| index)
                    .collect();
                match matches.len() {
                    1 => Ok(matches[0]),
                    0 => Err(format!("No cell has id \"{id}\".")),
                    count => Err(format!(
                        "{count} cells share id \"{id}\"; address one by cell_index instead."
                    )),
                }
            }
            (None, Some(index)) => {
                if index < self.cells.len() {
                    Ok(index)
                } else {
                    Err(format!(
                        "cell_index {index} is out of range; the notebook has {} cells.",
                        self.cells.len()
                    ))
                }
            }
        }
    }

    fn replace(
        &mut self,
        cell_id: Option<&str>,
        cell_index: Option<usize>,
        source: Option<&str>,
    ) -> Result<String, String> {
        let Some(source) = source else {
            return Err("Replacing a cell needs its new source.".to_string());
        };
        let index = self.locate(cell_id, cell_index)?;
        let mut decoded: Map<String, Value> = serde_json::from_str(self.cells[index].raw.get())
            .map_err(|_| "That cell could not be read.".to_string())?;
        decoded.insert("source".to_owned(), source_lines(source));
        // Outputs describe one execution of one source; once the source changes they describe
        // something that no longer exists, and nothing here can re-run the cell (D5).
        let cleared = if self.cells[index].cell_type == "code" {
            let had = decoded
                .get("outputs")
                .and_then(Value::as_array)
                .is_some_and(|outputs| !outputs.is_empty());
            decoded.insert("outputs".to_owned(), Value::Array(Vec::new()));
            decoded.insert("execution_count".to_owned(), Value::Null);
            had
        } else {
            false
        };
        self.cells[index] = build_cell(decoded)?;
        Ok(if cleared {
            format!("Replaced cell {index} and cleared its outputs")
        } else {
            format!("Replaced cell {index}")
        })
    }

    fn insert(
        &mut self,
        cell_id: Option<&str>,
        cell_index: Option<usize>,
        source: Option<&str>,
        cell_type: Option<&str>,
        position: Option<&str>,
    ) -> Result<String, String> {
        let cell_type = cell_type.unwrap_or("code");
        if cell_type != "code" && cell_type != "markdown" && cell_type != "raw" {
            return Err(format!(
                "cell_type must be code, markdown, or raw, not \"{cell_type}\"."
            ));
        }
        let source = source.unwrap_or("");
        let at = match (cell_id, cell_index) {
            // Inserting into an empty notebook, or explicitly at the end, has no cell to address.
            (None, None) if self.cells.is_empty() => 0,
            (None, None) => match position {
                Some("start") => 0,
                Some("end") | None => self.cells.len(),
                Some(other) => {
                    return Err(format!(
                        "Without a cell address, position must be start or end, not \"{other}\"."
                    ))
                }
            },
            _ => {
                let index = self.locate(cell_id, cell_index)?;
                match position {
                    Some("before") => index,
                    Some("after") | None => index + 1,
                    Some(other) => {
                        return Err(format!(
                        "With a cell address, position must be before or after, not \"{other}\"."
                    ))
                    }
                }
            }
        };

        let mut decoded = Map::new();
        if self.uses_cell_ids {
            decoded.insert("id".to_owned(), Value::String(new_cell_id()));
        }
        decoded.insert("cell_type".to_owned(), Value::String(cell_type.to_owned()));
        decoded.insert("metadata".to_owned(), Value::Object(Map::new()));
        decoded.insert("source".to_owned(), source_lines(source));
        if cell_type == "code" {
            decoded.insert("outputs".to_owned(), Value::Array(Vec::new()));
            decoded.insert("execution_count".to_owned(), Value::Null);
        }
        self.cells.insert(at, build_cell(decoded)?);
        Ok(format!("Inserted a {cell_type} cell at {at}"))
    }

    fn delete(
        &mut self,
        cell_id: Option<&str>,
        cell_index: Option<usize>,
    ) -> Result<String, String> {
        let index = self.locate(cell_id, cell_index)?;
        self.cells.remove(index);
        Ok(format!("Deleted cell {index}"))
    }

    /// Writes untouched cells and every other top-level member as the exact bytes they arrived as.
    /// Top-level key order is the one thing not preserved -- `serde_json`'s map is sorted, and with
    /// four keys the cost is a few lines, so they go out in nbformat's documented order (D4).
    fn render(&self) -> String {
        let mut out = String::from("{\n \"cells\": [\n");
        for (index, cell) in self.cells.iter().enumerate() {
            out.push_str("  ");
            out.push_str(cell.raw.get());
            if index + 1 < self.cells.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str(" ],\n");
        for name in ["metadata", "nbformat", "nbformat_minor"] {
            if let Some((_, raw)) = self.members.iter().find(|(key, _)| key == name) {
                out.push_str(&format!(" \"{name}\": {},\n", raw.get()));
            }
        }
        for (name, raw) in &self.members {
            if matches!(name.as_str(), "metadata" | "nbformat" | "nbformat_minor") {
                continue;
            }
            out.push_str(&format!(" \"{name}\": {},\n", raw.get()));
        }
        // Trim the final separator rather than tracking which member is last through two loops.
        while out.ends_with(",\n") {
            out.truncate(out.len() - 2);
            out.push('\n');
        }
        out.push_str("}\n");
        out
    }
}

/// nbformat stores source as one string per line, each keeping its newline except the last.
fn source_lines(source: &str) -> Value {
    if source.is_empty() {
        return Value::Array(Vec::new());
    }
    let mut lines: Vec<Value> = Vec::new();
    let mut rest = source;
    while let Some(position) = rest.find('\n') {
        lines.push(Value::String(rest[..=position].to_owned()));
        rest = &rest[position + 1..];
    }
    if !rest.is_empty() {
        lines.push(Value::String(rest.to_owned()));
    }
    Value::Array(lines)
}

fn build_cell(decoded: Map<String, Value>) -> Result<Cell, String> {
    let id = decoded.get("id").and_then(Value::as_str).map(str::to_owned);
    let cell_type = decoded
        .get("cell_type")
        .and_then(Value::as_str)
        .unwrap_or("code")
        .to_owned();
    let text = serde_json::to_string_pretty(&Value::Object(decoded))
        .map_err(|_| "That cell could not be written back.".to_string())?;
    let raw = RawValue::from_string(text)
        .map_err(|_| "That cell could not be written back.".to_string())?;
    Ok(Cell { raw, id, cell_type })
}

/// nbformat 4.5 requires a cell id of 1-64 characters from a restricted alphabet, unique within the
/// notebook. A UUID's first eight characters satisfy that and match what Jupyter itself writes.
fn new_cell_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_owned()
}

#[cfg(test)]
#[path = "notebook_tests.rs"]
mod tests;
