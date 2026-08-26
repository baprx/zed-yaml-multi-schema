//! US1 tests: annotated block validation scoped to its block only.

mod common;

use std::path::Path;

use common::{FakeFetcher, LOCAL_SCHEMA};

const ROOT: &str = "/repo";

#[test]
fn annotated_block_validates_against_local_schema() {
    let mut fetcher = FakeFetcher::new();
    fetcher.add_local("/repo/schemas/test.schema.json", LOCAL_SCHEMA);
    let mut server = fetcher.server(Path::new(ROOT));

    // Valid values -> no diagnostics.
    server.on_change(
        "# $schema=./schemas/test.schema.json\ntest:\n  enabled: true\n  elements: [1, A]\n  product: anch\n  version: 1\n",
    );
    assert!(server.diagnostics().is_empty());

    // Introduce a violation -> a diagnostic scoped to the annotated block.
    server.on_change(
        "# $schema=./schemas/test.schema.json\ntest:\n  enabled: not-a-bool\n  elements: [1]\n  product: anch\n  version: 1\n",
    );
    let diags = server.diagnostics();
    assert!(!diags.is_empty());
    // All diagnostics land within the `test` block (start_line == 1).
    for d in diags {
        assert_eq!(d.start_line, 1);
    }
}

#[test]
fn unannotated_blocks_produce_no_schema_diagnostics() {
    let fetcher = FakeFetcher::new();
    let mut server = fetcher.server(Path::new(ROOT));
    server.on_change("kubernetes:\n  kind: ConfigMap\n  data:\n    k: v\n");
    assert!(server.diagnostics().is_empty());
}
