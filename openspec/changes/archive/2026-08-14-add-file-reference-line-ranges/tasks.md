## 1. Domain

- [x] 1.1 Add optional `start_line`/`end_line` to `FileReference` with accessors, keeping every existing field and its validation intact
- [x] 1.2 Validate the range in the constructor: both bounds present or both absent, start at least 1, end not before start; add a dedicated domain error variant rather than reusing an existing one
- [x] 1.3 Change the `FileReferenceSet` dedup key from path to (path, start, end), keeping `MAX_FILE_REFERENCES` as the overall ceiling and keeping the duplicate error informative about which reference collided
- [x] 1.4 Update every existing construction site of `FileReference` in production and test code for the widened constructor

## 2. Prompt assembly

- [x] 2.1 Slice the file content in `compose_prompt` to the requested range, clamping an end past the last line and yielding an empty region for a start past the last line
- [x] 2.2 Label the injected block with the range it actually covers, and prefix each injected line with its 1-based source position
- [x] 2.3 Leave whole-file injection byte-identical to its current output for references carrying no range

## 3. Command and persistence boundary

- [x] 3.1 Carry `startLine`/`endLine` through the `send_message` DTO and its mapper
- [x] 3.2 Confirm the JSON persistence round-trips the new fields and that rows written without them restore as whole-file references — no schema migration

## 4. Frontend types and parsing

- [x] 4.1 Add `startLine`/`endLine` to `ChatFileReference` in the types module and to the contract re-export; extend the conformance assertions
- [x] 4.2 Parse a trailing `:start-end` or `:line` suffix off the mention token, accepting only digits so a path containing a colon stays a path
- [x] 4.3 Query candidate search with the path portion only, so completion survives while the range is being typed
- [x] 4.4 Build the attached reference from the parsed path and range, deriving its `id` from both so two regions of one file get distinct identities

## 5. Frontend display and removal

- [x] 5.1 Show the range on the composer chip and on the message-history chip; leave an unranged chip visually unchanged
- [x] 5.2 Move chip removal from path-keyed to identity-keyed in the composer, the layout model, and `MessageItem`
- [x] 5.3 Add the localized strings for the range label across all locale files
- [x] 5.4 Keep `ChatInputBox.tsx` under the 300-line cap — parsing belongs in the mention hook

## 6. Tests

- [x] 6.1 Rust: range validation accepts valid ranges and rejects one-sided, zero, negative, and inverted bounds
- [x] 6.2 Rust: two regions of one file coexist; an exact duplicate is still rejected; two unranged references to one path are still rejected
- [x] 6.3 Rust: injection contains only the requested lines, labelled with the range and with 1-based positions
- [x] 6.4 Rust: an end past the last line clamps; a start past the last line yields an empty region without error
- [x] 6.5 Rust: an unranged reference injects byte-identically to the pre-change output
- [x] 6.6 Frontend: mention suffix parsing — range, single line, no suffix, malformed suffix, and a path containing a colon
- [x] 6.7 Frontend: candidate search receives the path portion while a range is being typed
- [x] 6.8 Frontend: removing one of two references to the same path leaves the other attached
- [x] 6.9 Frontend: chips render the range, and unranged chips render without decoration

## 7. Verification

- [x] 7.1 `npm run lint:ci`
- [x] 7.2 `npm run test`
- [x] 7.3 `npm run build`
- [x] 7.4 `npm run contracts:check`
- [x] 7.5 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 7.6 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 7.7 `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 7.8 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 7.9 `npx playwright test` — composer behavior change
- [x] 7.10 `openspec validate add-file-reference-line-ranges --strict`
- [x] 7.11 `openspec validate --specs --strict`
- [x] 7.12 Confirm a message persisted before this change still restores and injects as a whole-file reference
