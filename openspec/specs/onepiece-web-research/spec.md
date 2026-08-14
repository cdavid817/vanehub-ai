# onepiece-web-research Specification

## Purpose
Enables OnePiece to search the public Web through DuckDuckGo and fetch selected pages through a guarded, bounded, citation-ready retrieval boundary.
## Requirements
### Requirement: DuckDuckGo-backed Web search
The system SHALL expose a bounded Web-search operation for OnePiece using a reviewed DuckDuckGo search adapter. Requests SHALL validate query length, locale/safety options, result count, deadline, and cancellation, and results SHALL contain normalized titles, URLs, snippets, ranks, and provider provenance.

#### Scenario: Search succeeds
- **WHEN** OnePiece submits a valid query while Web-search readiness and policy pass
- **THEN** the system SHALL return a bounded ordered result set with normalized source metadata and an explicit truncation indicator

#### Scenario: Search provider fails
- **WHEN** DuckDuckGo times out, rate-limits, rejects, or returns an invalid response
- **THEN** the system SHALL return a stable safe error category without fabricating results or silently switching to another provider

### Requirement: Guarded page fetching
The system SHALL fetch selected HTTP(S) pages only after URL normalization, scheme validation, DNS/address safety checks, redirect revalidation, content-type admission, response-size limits, decompression limits, timeout, and cancellation checks. Fetching SHALL use an isolated client without ambient browser cookies, host credentials, arbitrary authorization headers, or local-file access.

#### Scenario: Fetch a public page
- **WHEN** a selected public HTTP(S) URL and every redirect pass policy
- **THEN** the system SHALL return bounded status, final URL, headers allowlist, extraction metadata, and admitted content

#### Scenario: DNS resolves to a blocked network
- **WHEN** the initial host or any redirect resolves to a denied loopback, private, link-local, metadata, or otherwise restricted address
- **THEN** the system SHALL reject the request before reading the protected response

#### Scenario: Response exceeds limits
- **WHEN** compressed or expanded response content exceeds a hard limit
- **THEN** the system SHALL stop reading, discard incomplete unsafe content, and return a limit-exceeded outcome

### Requirement: Bounded content extraction
The Web-fetch operation SHALL extract supported textual page content into a bounded structured result with title, canonical/final URL, media type, capture time, text, and truncation metadata. Unsupported binary or active content SHALL not be returned as executable markup; a separately admitted download SHALL become an Artifact when explicitly requested and permitted.

#### Scenario: Extract an HTML article
- **WHEN** a fetched HTML page is admitted
- **THEN** the system SHALL return readable bounded text and source metadata without executing page scripts

#### Scenario: Fetch a supported document
- **WHEN** a fetched non-HTML document is supported by an available bounded extractor
- **THEN** the system SHALL return extracted content with the original media type and provenance

### Requirement: Citation-ready provenance
Every search and fetch result SHALL preserve sufficient safe provenance for OnePiece to cite the exact source, including normalized URL, final URL when redirected, provider, title when available, and capture time. The system SHALL distinguish search snippets from fetched page content.

#### Scenario: OnePiece uses a search snippet without fetching
- **WHEN** only a DuckDuckGo result snippet is available
- **THEN** the result SHALL identify it as provider-supplied snippet evidence rather than fetched page content

#### Scenario: Page content is fetched
- **WHEN** a result URL is fetched successfully
- **THEN** the returned evidence SHALL identify the final source URL and capture metadata used for the content

### Requirement: Web research is isolated from arbitrary network authority
Availability of Web search or fetch SHALL NOT grant network access to the general shell, code-execution sandbox, OCR worker, Artifact renderer, or delegated CLI child tools. Each capability SHALL retain its own explicit network policy.

#### Scenario: Code sandbox attempts an HTTP request
- **WHEN** code running in the independent code-execution sandbox attempts network access while its policy denies network
- **THEN** the request SHALL remain denied even though OnePiece has an eligible Web-fetch tool

### Requirement: Web runtime does not impersonate native retrieval
The Web/mock adapter SHALL preserve search/fetch contracts through deterministic fixtures or an explicit backend-required outcome and SHALL NOT claim to have fetched a live page unless an actual configured Web backend performed the request.

#### Scenario: Mock search
- **WHEN** Web/mock mode simulates Web research
- **THEN** the result SHALL be marked as mock data and SHALL not be persisted as live-source evidence
