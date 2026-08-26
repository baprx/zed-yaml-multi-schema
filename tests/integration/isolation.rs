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
    // All diagnostics are scoped to the `test` block; `kubernetes` is clean.
    assert!(!diags.is_empty());
    for d in diags {
        assert_eq!(d.start_line, 1, "diagnostic leaked out of test block");
    }
}
