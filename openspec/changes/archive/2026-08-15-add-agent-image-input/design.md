# Design

## D1. Why this unlocks work already paid for

The expensive parts are already built and shipped: a managed Playwright sidecar that can screenshot
a page, a checksum-verified PDFium renderer, and a content-addressed Artifact store that already
moves binary between tools by id. What is missing is three lines of wire format — both provider
modules build text-only content blocks.

That gap is why `ocr` exists in the shape it does. OCR is the workaround for a model that cannot
see: it converts pixels to characters and throws away everything else. For "read the text in this
scan" that is correct. For "is this layout broken", "did the dialog render off-screen", "what
does this chart show", it destroys exactly the information being asked about.

## D2. One internal representation, two translations

Both providers get their image shape from one internal image value (bytes, media type,
dimensions), translated at the same place `tool_definition_to_json` already translates tools. The
alternative — an image path per provider module — would double every bound check and every
redaction rule, and the second copy is the one that would drift.

The regression risk here is entirely in the *text-only* path, which must stay byte-identical. That
is why it is an explicit scenario rather than an assumption: every existing provider test asserts
on request bodies, and a stray empty `content` array would break them all at once.

## D3. Capability gating is a predicate, not a try-and-see

`onepiece-native-agent` already establishes that capabilities are not inferred from unknown model
identifiers, and model discovery already merges reviewed catalog metadata. Image support follows
the same rule: reviewed metadata says yes, or the answer is no.

Try-and-see would be worse than useless here. A provider that rejects an image-bearing request
fails the whole generation, after the user has already waited — and the failure text varies by
vendor, so it cannot be reliably distinguished from a real error.

The consequence is that image-capable tools must degrade rather than fail (`Image-capable tool on
a text-only model`): a screenshot on a text-only model still produces its Artifact, it just does
not attach the image. Failing instead would make a model choice silently break a tool.

## D4. Bounds, and why downscale-then-refuse

| Bound | Behavior | Rationale |
| --- | --- | --- |
| Pixel dimensions | Downscale | Providers downscale server-side anyway; doing it locally makes the token cost predictable and the upload smaller. |
| Encoded bytes after downscale | Refuse | If it is still too big after downscaling, it is not an image the model was meant to read. |
| Images per request | Refuse | Silently dropping the third image would answer a question about it with confident nonsense. |

Downscaling is reported in the result. A model reasoning about pixel positions needs to know it is
not looking at the original resolution.

## D5. Bytes never enter logs or the transcript

The redaction rules for this capability are the same ones the extended tools already follow, with
one addition that matters: an image is not "private content" the way a prompt is, it is *bulk* —
a single screenshot base64-encodes to more than the entire durable log line budget. So logs carry
hash, media type, dimensions, and byte count, and the transcript carries an Artifact reference.

This also means the Artifact store is the transport, which it already is for tool-to-tool binary
(`onepiece-artifact-publishing`). No new persistence is introduced.

## D6. Token accounting cannot be estimated

The existing estimation fallback derives tokens from character counts. That is meaningless for an
image: its cost depends on the provider's own tiling of its dimensions. So an image-bearing request
without provider-reported usage reduces reported coverage rather than guessing — consistent with
how `onepiece-native-agent` already handles a provider that omits usage.

## D7. Open question: the resize dependency

Decoding and downscaling needs an image crate, which is a new third-party dependency in a
repository with a supply-chain governance capability. That review is a prerequisite, not an
implementation detail, and the alternative — sending only images already within bounds and
refusing everything else — is a legitimate smaller first delivery if the review does not land.
