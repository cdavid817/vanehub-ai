## ADDED Requirements

### Requirement: Image content-block translation
The provider runtime SHALL translate an image carried by a user turn or a tool result into the image content shape required by the session's `interface_format`, using one shared internal representation rather than a per-provider image path. A request that carries no image SHALL be byte-identical to what the same request produced before image support existed.

#### Scenario: Anthropic format translation
- **WHEN** a request carrying an image is built for `interface_format` `anthropic`
- **THEN** the image SHALL be declared using that format's image content block

#### Scenario: OpenAI-compatible format translation
- **WHEN** a request carrying an image is built for `interface_format` `openai-compatible`
- **THEN** the image SHALL be declared using that format's image content shape

#### Scenario: Text-only requests are unchanged
- **WHEN** a request carries no image
- **THEN** its body SHALL be identical to the body produced before image support was added
