//! US3 tests: remote and local resolution with graceful fallback.

mod common;

use std::path::Path;

use common::{FakeFetcher, REMOTE_SCHEMA};

const ROOT: &str = "/repo";

#[test]
fn remote_https_schema_resolves_and_applies() {
    let mut fetcher = FakeFetcher::new();
    fetcher.add_remote(
        "https://example.com/argocd-apps/values.schema.json",
        REMOTE_SCHEMA,
    );

    let mut server = fetcher.server(Path::new(ROOT));
    server.on_change(
        "# $schema=https://example.com/argocd-apps/values.schema.json\nargocd-apps:\n  enabled: true\n",
    );
    // Valid against remote schema -> no diagnostics.
    assert!(server.diagnostics().is_empty());

    server.on_change(
        "# $schema=https://example.com/argocd-apps/values.schema.json\nargocd-apps:\n  enabled: not-a-bool\n",
    );
    assert!(!server.diagnostics().is_empty());
}

#[test]
fn unreachable_schema_yields_warning_and_keeps_file_editable() {
    let fetcher = FakeFetcher::new();
    let mut server = fetcher.server(Path::new(ROOT));
    server.on_change(
        "# $schema=https://offline.example/values.schema.json\nchart:\n  enabled: true\n",
    );
    let diags = server.diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, "warning");
    assert!(diags[0].message.contains("offline.example"));

    // The file remains fully editable: a later, valid change with no annotation
    // produces no diagnostics.
    server.on_change("other:\n  x: 1\n");
    assert!(server.diagnostics().is_empty());
}
