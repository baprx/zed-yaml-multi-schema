# Zed YAML Multi-Schema

A [Zed](https://zed.dev) extension that applies **multiple JSON Schemas within a
single YAML file**, each keyed by a `# $schema=<ref>` comment directly above a
block. This fixes the gap where only one schema per file was possible, which is
incompatible with deploying Helm charts in umbrella mode (where the top-level key
is not the file root, and several charts share one `values.yaml`).

## Features

- **Per-block validation**: each annotated block is validated against its own
  schema, scoped so diagnostics never leak across block boundaries.
- **Per-block completion**: property-name completions from the governing schema.
- **Local and remote schemas**: references may be local relative paths or remote
  HTTPS URLs.
- **Graceful fallback**: an unreachable or malformed schema produces a clear
  diagnostic and never blocks editing.
- **Non-intrusive**: the extension only analyzes and validates; it never rewrites
  or reorders your YAML.

## Installation

Install as a **dev extension** while developing, or from the extensions registry
once published.

**Dev extension (from source):**

1. Ensure [Rust](https://rustup.rs) is installed (Zed installs the `wasm32-wasip2`
   target automatically).
2. Build the bundled language-server binary and place it on your `PATH`:
   ```bash
   cargo build --release --features lsp-bin
   cp target/release/zed-yaml-multi-schema-lsp ~/.local/bin/
   ```
   (`language_server_command` looks up `zed-yaml-multi-schema-lsp` on the `PATH`.)
3. In Zed: open the Extensions panel (`zed: extensions`), click
   **Install Dev Extension**, and select this repository's directory.
4. Reload or reopen a YAML file; the extension attaches to YAML and provides
   per-block diagnostics and completions.

**From the registry:** search for "Zed YAML Multi-Schema" in the Extensions panel
and click **Install** (once published).

## Usage

Place a `# $schema=<ref>` comment directly above a top-level block:

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
  kind: ConfigMap # unannotated blocks are supported without a schema
```

- The `$schema` reference is either a **local relative path** (resolved against
  the worktree root) or a **remote HTTPS URL**.
- Each annotated block is governed by its own schema; unannotated blocks are
  ignored.

## Development

```bash
cargo build          # build the extension + LSP server
cargo test           # unit + integration tests (test-first)
cargo clippy --all-targets -- -D warnings
cargo fmt -- --check
prek                 # or: pre-commit run --all-files
```

### Architecture

The extension ships a bundled LSP server (`src/main.rs`) launched via
`language_server_command`. The core logic lives in pure, testable modules:

- `src/document.rs` — parse YAML and map `# $schema=` annotations to block ranges
- `src/resolver.rs` — resolve local and remote schemas with caching
- `src/validator.rs` — JSON Schema validation (draft detected from `$schema`)
- `src/completion.rs` — derive completions from a schema
- `src/server.rs` — per-block diagnostics/completions facade (testable without WASM)

## License

MIT
