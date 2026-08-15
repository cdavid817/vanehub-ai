## Why

A Jupyter notebook is the one common source file the native agent can neither read usefully nor edit at all.

Reading burns the context window on noise. A four-cell notebook with one plot is 122 KB on disk; the `file` tool's per-line cap delivers about 4,500 characters of it, of which roughly 135 are the actual code — the rest is JSON scaffolding plus two thousand characters of truncated base64 from a single output image. That is a 3% signal ratio, and it gets worse with every executed cell.

Editing is not merely awkward, it is effectively impossible. Cell source is stored as a JSON array of escaped strings, so changing one line means the model must produce `    "df = pd.read_csv(\'data.csv\')\\n",` — exact escaping, exact indentation, and a separate array element per line. `edit` matches exact strings against the raw file, so a multi-line change spans several array elements and cannot be expressed as one unique match. Nothing in the toolset addresses this.

The result is that a data or ML repository is a place this agent cannot work, for a reason that has nothing to do with the difficulty of the task it was asked to do.

## What Changes

- Add a `notebook` tool that reads a notebook as cells — index, id, type, source, and a bounded summary of each output — rather than as raw JSON.
- Never put an output image's bytes in the read result. An image output is reported by media type and size, so the model learns a plot exists without paying 120 KB for it.
- Let the same tool replace, insert, and delete a cell, addressing it by the cell id a notebook carries or by its index.
- Preserve every byte of every cell the edit does not touch, and of the notebook's own metadata, so a one-cell change produces a one-cell diff.
- Clear a code cell's outputs and execution count when its source changes, because outputs that no longer correspond to their source are worse than absent ones.
- Refuse a file that is not a readable notebook, rather than corrupting it by writing a guess back.

## Capabilities

### New Capabilities

- `agent-notebook-editing`: Reading a notebook as cells and editing it a cell at a time, without the model handling notebook JSON.

## Impact

- One tool joins the ordinary catalog, appended after the existing entries so the prompt-cache prefix of every native generation is unchanged.
- Plan mode offers its read operation only, matching how the `file` tool is narrowed there.
- No new package dependency: `serde_json`'s `RawValue` is already available in this build and is what makes byte-preserving rewrites possible.
- No new persistence, no Tauri command, and no change to how any existing tool behaves. A notebook stays readable by `file` for a caller that wants the raw JSON.
