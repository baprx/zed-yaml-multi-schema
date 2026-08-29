//! US3 tests: remote and local resolution with graceful fallback.

mod common;

use std::path::Path;
use std::sync::{Arc, Mutex};

use zed_yaml_multi_schema::resolver::SchemaFetcher;
use zed_yaml_multi_schema::server::YamlServer;

use common::{FakeFetcher, REMOTE_SCHEMA};

const ROOT: &str = "/repo";

/// Fetcher whose remote map can be populated lazily, to test retry-on-change.
struct GrowingFetcher {
    remote: Arc<Mutex<Vec<String>>>,
}

impl GrowingFetcher {
    fn new() -> Self {
        Self {
            remote: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl SchemaFetcher for GrowingFetcher {
    fn read_local(&self, _path: &str) -> Result<String, String> {
        Err("no local".to_string())
    }
    fn fetch_remote(&self, url: &str) -> Result<String, String> {
        if self.remote.lock().unwrap().contains(&url.to_string()) {
            Ok(REMOTE_SCHEMA.to_string())
        } else {
            Err(format!("unavailable: {url}"))
        }
    }
}

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

#[test]
fn previously_failed_reference_is_retried_on_subsequent_change() {
    let fetcher = GrowingFetcher::new();
    // Keep our own handle to the shared state before `fetcher` is moved
    // into the Arc that YamlServer owns.
    let remote = Arc::clone(&fetcher.remote);
    let mut server = YamlServer::new(Arc::new(fetcher), Path::new(ROOT));

    let url = "https://example.com/chart/values.schema.json";
    server.on_change(&format!("# $schema={url}\nchart:\n  enabled: true\n"));
    let diags = server.diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, "warning");

    remote.lock().unwrap().push(url.to_string());
    server.on_change(&format!("# $schema={url}\nchart:\n  enabled: true\n"));
    assert!(
        server.diagnostics().is_empty(),
        "failed reference was not re-attempted: {:?}",
        server.diagnostics(),
    );
}
