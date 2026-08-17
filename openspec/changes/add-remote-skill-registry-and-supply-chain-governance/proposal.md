## Why

The effective Skill runtime defines a Registry layer, but VaneHub has no secure way to discover, verify, install, update, revoke, or roll back remotely published Skill packages. Adding network installation without repository security and immutable local snapshots would expose every Skill load, configuration, Overlay, and bundled tool boundary to supply-chain compromise.

## What Changes

- Add first-party and explicitly trusted custom Skill registry sources with stable ids, HTTPS endpoints, root trust metadata, optional credential presence, enablement, health, and refresh state.
- Define signed, versioned registry metadata with root rotation, expiry, rollback/freeze protection, delegated publisher namespaces, package hashes, sizes, compatibility, risk metadata, and revocation state.
- Add bounded catalog search, filtering, details, version history, update checks, and cached read-only browsing without executing or installing catalog content.
- Download packages through the VaneHub network/proxy boundary into a content-addressed quarantine cache, verify metadata/signatures/hashes, safely extract and validate package structure, and install an immutable snapshot into the Registry layer through an atomic transaction.
- Add explicit install/update/downgrade/rollback/uninstall previews and confirmations. Updates are discovered automatically but never installed automatically in the first release.
- Parse the package's normalized permission manifest during quarantine validation and show an install-time permission review plus a version-to-version permission diff. Any authority expansion requires explicit confirmation and can never be silently auto-applied.
- Keep package provenance trust separate from Overlay trust, configuration values, permission grants, and executable Skill tool trust; a verified package does not receive operational authority.
- Handle signed security revocations with fail-closed runtime eligibility, retained evidence, recovery guidance, and no silent deletion of user data or Overlay history.
- Add Registry browsing and installed-version governance to the Skills settings page through the frontend service boundary, with honest Web behavior and unified redacted operation logs.

## Capabilities

### New Capabilities

- `skill-registry-management`: Defines registry source trust, signed metadata, catalog discovery, package verification, quarantine, installation, update, rollback, revocation, cache, and offline behavior.

### Modified Capabilities

- `skill-management`: Adds immutable Registry-layer packages, provenance, version/install state, effective-resolution integration, and safe lifecycle operations.
- `settings-skill-management-ui`: Adds registry catalog, source management, install preview, update/rollback, revocation, provenance, and operation-status interfaces.
- `software-supply-chain-security`: Extends product supply-chain requirements to remotely distributed Skill metadata and packages.

## Impact

- Desktop/native: adds a Registry subdomain inside the existing `tooling` bounded context, with registry clients, trust metadata verification, bounded downloader/cache/extractor, package validator, SQLite repositories, asynchronous operations, Tauri commands, and unified logs.
- Frontend: extends `AgentService`, Tauri adapter, and Web adapter with registry/catalog/install governance contracts; React components do not call native commands directly.
- Storage: adds application-owned immutable Registry snapshots, quarantine/cache directories, and additive SQLite state for sources, metadata witnesses, installed versions, operations, revocations, and rollback candidates.
- Network: uses existing VaneHub-managed HTTPS/proxy settings, strict redirects and limits, conditional refresh, offline cache rules, and credential-store isolation for authenticated registries.
- Security: adds a reviewed repository-security dependency and package-archive parser; verified provenance remains independent from runtime permission or executable-content trust.
