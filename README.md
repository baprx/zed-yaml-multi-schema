# YAML Multi-Schema

A [Zed](https://zed.dev) extension that applies **multiple JSON Schemas within a
single YAML file**, each keyed by a `# $schema=<ref>` comment directly above a
block. This fixes the gap where only one schema per file was possible, which is
incompatible with deploying Helm charts in umbrella mode (where the top-level key
is not the file root, and several charts share one `values.yaml`).

## Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [How it works](#how-it-works)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [License](#license)

## Features

- **Per-block validation**: each annotated block is validated against its own
  schema, scoped so diagnostics never leak across block boundaries.
- **Per-block completion**: property-name completions from the governing schema,
  offered while you type, with keys already present in the map filtered out.
- **Local and remote schemas**: references may be local relative paths or remote
  HTTPS URLs.
- **Graceful fallback**: an unreachable or malformed schema produces a clear
  warning and never blocks editing.

## Installation

### From the registry (recommended)

Search for "YAML Multi-Schema" in Zed's Extensions panel and click **Install** —
no manual steps. The extension downloads the matching `yaml-multi-schema-lsp`
binary for your platform from this repository's GitHub release automatically
(macOS arm64/x64 and Linux x64) and keeps it in a versioned cache.

### Dev extension (from source)

Only needed when developing the extension itself:

1. Ensure [Rust](https://rustup.rs) is installed (Zed installs the `wasm32-wasip2`
   target automatically).
2. Build the language-server binary and place it on your `PATH`:
   ```bash
   cargo build --release --features lsp-bin
   cp target/release/yaml-multi-schema-lsp ~/.local/bin/
   ```
   (A binary on the `PATH` is preferred over the downloaded one, so dev builds
   take effect without touching the cache.)
3. In Zed: open the Extensions panel (`zed: extensions`), click
   **Install Dev Extension**, and select this repository's directory.
4. Reload or reopen a YAML file; the extension attaches to YAML and provides
   per-block diagnostics and completions.

## Usage

### Annotating blocks

Place a `# $schema=<ref>` comment directly above a top-level block (only blank
or comment lines may sit between the annotation and the key):

```yaml
# $schema=./schemas/test.schema.json
test:
  enabled: true
  elements:
    - 1
    - A

# $schema=https://raw.githubusercontent.com/traefik/traefik-helm-chart/refs/heads/master/traefik/values.schema.json
traefik:
  enabled: true
  image:
    registry: example.com
    repository: traefik
    tag: 1.1.1

kubernetes:
  kind: ConfigMap
```

## How it works

- The extension ships a language server that attaches to YAML buffers and
  revalidates on every open and change.
- Each annotated block is validated independently, so a problem in one chart's
  section never flags another's. Validation errors point at the specific
  offending key or list element, not the whole block.
- Schema drafts are detected from each schema's `$schema` keyword, so both
  draft-07 (common in Helm charts) and 2020-12 schemas validate correctly.
  Unknown keywords are tolerated rather than rejected.
- A schema that cannot be resolved or parsed yields a **warning** on the block;
  a value that violates its schema yields an **error** on the offending line.
  Neither ever prevents editing.

## Development

```bash
cargo build          # build the extension + LSP server
cargo test           # unit + integration tests (test-first)
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
```

## License

Distributed under the [GPL-3.0](LICENSE.txt) license.
