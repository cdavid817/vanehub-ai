## Context

The previous repository state had a minimally recognizable `V` icon, a Windows-only bundle reference, several competing candidate designs, and Windows-specific PowerShell generators. VaneHub AI ships a Tauri desktop application and a browser-accessible UI, while release assets may also be consumed by mobile platforms and application stores. The icon therefore needs one durable brand language, deterministic outputs, and explicit optical treatment from 16px through high-resolution store artwork.

This change is visual and packaging-oriented. It does not introduce runtime behavior, Tauri commands, service interfaces, persistence, or agent-specific branches, so the existing React service, Tauri adapter, Web adapter, and Rust/native boundaries remain unchanged.

## Goals / Non-Goals

**Goals:**

- Preserve the recognizable VaneHub `V` while expressing agent coordination and a shared workspace.
- Provide legible optical variants for 16–32px and detailed artwork for 48–512px and store surfaces.
- Generate repeatable Windows, macOS, Linux, iOS, Android, and Web assets from canonical vector sources.
- Integrate the correct formats into Tauri and browser metadata.
- Leave one cross-platform generation command and remove obsolete candidate workflows.

**Non-Goals:**

- Redesign the in-product Lucide icon system or semantic UI tokens.
- Add an iOS or Android application runtime.
- Change frontend services, native commands, agent behavior, or persistence.
- Automate subjective brand approval or replace human visual review.

## Decisions

### Decision: Preserve and simplify the V mark

The canonical mark keeps the existing `V` silhouette, uses its rounded arm endpoints as implicit agent nodes, and retains one explicit lower Hub plus a restrained orchestration orbit. A flat deep-navy base, off-white mark, and single cyan accent align with the application's futuristic and minimal themes while keeping the silhouette dominant.

Alternative considered: replace the icon with a generic node graph or new monogram. This was rejected because it would discard the only existing brand cue and reduce recognition continuity.

### Decision: Maintain master and compact optical SVGs

`source/app-icon.svg` is the master for 48px and larger surfaces and retains small endpoint nodes plus the explicit Hub. `source/app-icon-compact.svg` removes endpoint-node detail for 16px, 24px, 32px, and favicon use. Android receives separate foreground and monochrome sources so adaptive masking does not crop the mark.

Alternative considered: resize one 512px raster to every target. This was rejected because fine detail collapses and line weights become inconsistent below 32px.

### Decision: Use a minimal flat surface treatment

The icon uses one solid background, one off-white foreground, and one cyan accent. Decorative inner rims, ambient auras, blur or offset shadows, glossy highlights, multistop material gradients, and the internal bridge are excluded. Depth comes from shape overlap and negative space rather than simulated material.

Alternative considered: retain the lightly dimensional glass treatment. This was rejected because the extra layers competed with the `V`, appeared less durable as a brand style, and added noise at medium sizes.

### Decision: Use deterministic vector generation instead of generative raster output

SVG geometry is the source of truth. The repository's installed Tauri CLI rasterizes platform assets, and a Node script assembles a mixed-optical-size ICO and copies Web outputs. Generated files are committed so packaging does not depend on regeneration during every build. Deterministic generation means stable source-derived artwork, dimensions, formats, and file layout; the Tauri ICNS writer may emit byte-different container metadata across runs without changing the rendered icon content.

Alternative considered: use an image model for the final icon. This was rejected because a brand mark requires exact repeatability, editability, monochrome support, and predictable pixel alignment.

### Decision: Use a cross-platform Node generator

`npm run icons:generate` invokes `scripts/generate-vanehub-icon.mjs`, which calls the locally installed Tauri CLI through Node and uses only Node standard-library APIs for file and ICO operations. This avoids a PowerShell-only maintenance path and keeps the workflow usable on Windows, macOS, and Linux.

Alternative considered: keep the legacy PowerShell generators. This was rejected because they recreate obsolete candidates and are not portable to all supported development hosts.

### Decision: Keep icon assets directly under `src-tauri/icons`

Canonical sources, generated platform assets, optical-size PNGs, raster exports, and documentation use the flat top-level groups `source/`, `generated/`, `optical/`, `raster/`, and `README.md`. Tauri references only files under `generated/`; Web metadata references copies under `public/`.

Alternative considered: retain a versioned `vanehub-v2/` layer. This was rejected because the repository now has a single authoritative icon system and no parallel version requires namespacing.

## Risks / Trade-offs

- [Visual symbolism can still be interpreted differently] → Preserve the `V` silhouette, document the intended semantics, and retain human review at representative sizes.
- [Vector detail or container metadata may vary across tool versions] → Pin the project Tauri CLI dependency, commit generated outputs, verify exact dimensions and ICO frame membership, and do not treat metadata-only ICNS hash changes as artwork drift.
- [Adaptive masks can crop artwork] → Use a dedicated Android foreground with a conservative safe area and inspect representative round and square outputs.
- [Sources and generated files can drift] → Maintain one `icons:generate` command and require regeneration plus verification whenever a source changes.
- [Committed multi-platform assets increase repository size] → Keep only authoritative outputs and remove legacy candidates and generators.

## Migration Plan

1. Add canonical SVG sources and the Tauri icon manifest under `src-tauri/icons/source/`.
2. Add the cross-platform Node generator and generate platform, optical, raster, and Web assets.
3. Update Tauri and browser metadata to reference the new outputs.
4. Remove old root-level candidates, backups, and PowerShell generators.
5. Verify dimensions, ICO frames, lint, build, automated tests, Rust checks, and OpenSpec validation.

Rollback uses version control to restore the previous bundle reference and icon assets; no user data or runtime migration is involved.

## Open Questions

None. Future brand studies can propose a separate change without reintroducing parallel generators into this capability.
