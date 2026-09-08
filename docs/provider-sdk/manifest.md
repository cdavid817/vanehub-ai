# Provider Manifest Schema 1

The canonical schema-1 representation is strict JSON. Built-in declarations are compiled into the application and validated before the production registry becomes available.

```json
{
  "schemaVersion": 1,
  "id": "fixture-cli",
  "name": "Fixture CLI",
  "runtime": "cli",
  "executables": ["fixture"],
  "capabilities": {
    "terminal": true,
    "resume": true,
    "structuredOutput": true,
    "images": false,
    "usage": true,
    "permissions": true,
    "modelSelection": true,
    "reasoning": false,
    "sandbox": false
  }
}
```

Unknown fields and versions, duplicate keys, unsupported capabilities, and inconsistent domain values are errors. `executables` accepts reviewed basenames only. A manifest cannot contain arguments, environment values, commands, hooks, scripts, URLs, paths, dynamic libraries, or entrypoints. Parsing only produces domain declarations: it performs no probe, process launch, install, download, or network access.

## Schema 2

Schema 2 is the data-only superset used by the providers added in `extend-cli-providers-with-acp`. It adds two fields and changes one:

```json
{
  "schemaVersion": 2,
  "id": "qwen-code",
  "name": "Qwen Code",
  "runtime": "cli",
  "executables": ["qwen"],
  "transports": ["terminal", "acp-stdio"],
  "lifecycle": "active",
  "capabilities": {
    "terminal": true,
    "resume": true,
    "structuredOutput": true,
    "images": false,
    "usage": "unavailable",
    "permissions": true,
    "modelSelection": false,
    "reasoning": false,
    "sandbox": false
  }
}
```

- `transports` lists the reviewed transports from `terminal`, `headless`, and `acp-stdio`. A provider with no managed transport (terminal only) is a legacy entry.
- `lifecycle` is `active` or `legacy`; a legacy provider declares no ACP or headless transport and is never installed by the application.
- `usage` becomes tri-state: `true`, `false`, or `"unavailable"`. Unavailable usage is reported to the UI as unavailable, never as zero.

Everything else keeps the schema-1 rules: no arguments, environment values, commands, hooks, scripts, URLs, paths, or entrypoints. The ACP entry flag, account profiles, and vendor extension method names are compiled provider definitions, not manifest data. Schema-1 documents for the original five providers parse unchanged.

Schema 1 and 2 do not authorize external manifests. An `external:` provider reference returns the classified `ExternalProviderUnsupported` result.
