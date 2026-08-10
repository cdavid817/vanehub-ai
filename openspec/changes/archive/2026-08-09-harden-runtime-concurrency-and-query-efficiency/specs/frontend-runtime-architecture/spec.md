## ADDED Requirements

### Requirement: A web-http runtime with no HTTP adapter fails loudly

When the runtime kind is `web-http` and a service provides no `webHttp` adapter, the runtime adapter selector SHALL throw an explicit error naming the missing adapter rather than falling back to the web-mock adapter. Falling back silently would serve fabricated data in a production HTTP deployment — the worst failure mode, since the application looks healthy but every read is fake.

#### Scenario: An HTTP deployment is selected but an adapter is missing

- **WHEN** the host sets the HTTP base URL (selecting the `web-http` runtime) and a service provides no `webHttp` adapter
- **THEN** constructing that service SHALL throw an error that names the missing adapter
- **AND** the application's bootstrap-failure handler SHALL surface it as a recovery panel rather than running on mock data

#### Scenario: An HTTP deployment has a complete adapter

- **WHEN** the `web-http` runtime is selected and a service provides a `webHttp` adapter
- **THEN** the selector SHALL use that adapter

### Requirement: Streaming events are coalesced before reaching the query cache

A frontend service that subscribes to high-frequency stream events SHALL buffer them and flush the batch through a single query-cache update per animation frame, rather than calling `setQueryData` once per event. The batch SHALL apply in a single array traversal so unchanged messages keep their reference identity.

#### Scenario: A burst of stream events arrives within one frame

- **WHEN** more than one stream event arrives before the next animation frame
- **THEN** the subscription SHALL coalesce them into one `setQueryData` call that applies them as a batch
- **AND** a terminal event (completed/failed/cancelled) SHALL flush the buffer immediately rather than waiting for the frame
