//! US2 isolation tests: diagnostics never leak across block boundaries.

mod common;

use std::path::Path;

use common::{FakeFetcher, LOCAL_SCHEMA};

const ROOT: &str = "/repo";

#[test]
fn error_in_one_block_does_not_affect_other_blocks() {
    let mut fetcher = FakeFetcher::new();
    fetcher.add_local("/repo/schemas/test.schema.json", LOCAL_SCHEMA);

    let mut server = fetcher.server(Path::new(ROOT));
    server.on_change(
        "# $schema=./schemas/test.schema.json\ntest:\n  enabled: bad\n\nkubernetes:\n  kind: ConfigMap\n",
    );

    let diags = server.diagnostics();
    // All diagnostics stay inside the `test` block (lines 1-2); none leak into
    // the `kubernetes` block below.
    assert!(!diags.is_empty());
    for d in diags {
        assert!(
            d.start_line >= 1 && d.start_line <= 2,
            "diagnostic leaked out of test block"
        );
    }
    // The `enabled` type error points at its own line (2); missing-required
    // errors point at the block start (1).
    assert!(diags
        .iter()
        .any(|d| d.start_line == 2 && d.message.contains("enabled")));
}
