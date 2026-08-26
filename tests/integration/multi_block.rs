//! US2 tests: multiple annotated blocks in one file (umbrella mode).

mod common;

use std::path::Path;

use common::{FakeFetcher, LOCAL_SCHEMA, REMOTE_SCHEMA};

const ROOT: &str = "/repo";

#[test]
fn each_annotated_block_uses_its_own_schema() {
    let mut fetcher = FakeFetcher::new();
    fetcher.add_local("/repo/schemas/test.schema.json", LOCAL_SCHEMA);
    fetcher.add_remote(
        "https://example.com/traefik/values.schema.json",
        REMOTE_SCHEMA,
    );

    let mut server = fetcher.server(Path::new(ROOT));
    server.on_change(
        "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  elements: [1]\n  product: anch\n  version: 1\n\n# $schema=https://example.com/traefik/values.schema.json\ntraefik:\n  enabled: true\n  image:\n    registry: example.com\n    repository: traefik\n    tag: \"1.1.1\"\n",
    );

    // Both blocks valid -> no diagnostics.
    assert!(server.diagnostics().is_empty());

    // Break `traefik` only (tag must be a string).
    server.on_change(
        "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  elements: [1]\n  product: anch\n  version: 1\n\n# $schema=https://example.com/traefik/values.schema.json\ntraefik:\n  enabled: true\n  image:\n    registry: example.com\n    repository: traefik\n    tag: 12345\n",
    );

    let diags = server.diagnostics();
    assert!(!diags.is_empty());
    // Diagnostics only for the `traefik` block (starts at line 8).
    for d in diags {
        assert!(d.start_line >= 8, "leaked diagnostic at {}", d.start_line);
    }
}
