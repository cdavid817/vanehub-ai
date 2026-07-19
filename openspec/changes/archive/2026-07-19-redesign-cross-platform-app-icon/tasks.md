## 1. Canonical Brand Sources

- [x] 1.1 Create the detailed master SVG while preserving the VaneHub `V`, agent endpoints, orchestration orbit, and restrained brand palette.
- [x] 1.2 Create the compact SVG for 16px, 24px, 32px, and favicon rendering with reduced detail density.
- [x] 1.3 Add Android adaptive foreground, monochrome source, background color, and Tauri icon manifest.
- [x] 1.4 Document the icon semantics, optical variants, directory layout, and regeneration command.

## 2. Cross-Platform Generation

- [x] 2.1 Add the cross-platform Node generator and expose it through `npm run icons:generate`.
- [x] 2.2 Generate Windows, macOS, Linux, iOS, Android, optical-size, and raster icon assets from the canonical sources.
- [x] 2.3 Assemble a Windows ICO containing compact 16px–32px frames and master 48px–256px frames.
- [x] 2.4 Copy deterministic favicon, Apple touch, 192px, and 512px Web assets into `public/`.

## 3. Runtime Integration and Cleanup

- [x] 3.1 Configure Tauri to use generated PNG, ICNS, and ICO assets for native bundles.
- [x] 3.2 Add favicon, theme color, Apple touch icon, and Web App Manifest metadata to the browser entry point.
- [x] 3.3 Flatten the authoritative icon layout directly under `src-tauri/icons/`.
- [x] 3.4 Remove obsolete candidate icons, backups, versioned wrappers, and legacy PowerShell generators.

## 4. Verification

- [x] 4.1 Inspect 16px, 24px, 32px, 48px, 64px, 128px, 256px, and 512px outputs for legibility and valid dimensions.
- [x] 4.2 Verify ICO frame membership, configured Tauri paths, Android adaptive outputs, and Web manifest assets.
- [x] 4.3 Run icon regeneration, frontend build, lint, frontend tests, Rust tests, Cargo check, and Cargo clippy.
- [x] 4.4 Run strict OpenSpec validation and verify implementation-to-artifact consistency.

## 5. Minimal Visual Refinement

- [x] 5.1 Simplify the master and compact SVGs to a flat three-color system and remove decorative effects and the internal bridge.
- [x] 5.2 Align Android adaptive and monochrome sources plus icon documentation with the minimal geometry.
- [x] 5.3 Regenerate all platform assets and visually inspect representative 16px, 32px, 64px, 256px, and 512px outputs.
- [x] 5.4 Re-run icon, build, lint, test, Rust, and strict OpenSpec verification for the refined artwork.
