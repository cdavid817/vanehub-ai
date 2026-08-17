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

Schema 1 does not authorize external manifests. An `external:` provider reference returns the classified `ExternalProviderUnsupported` result.
