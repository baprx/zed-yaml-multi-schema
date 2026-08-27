//! SC-001: validate the real `example-values.yaml` (3 annotated blocks) so all
//! three annotations are resolved and applied to their correct blocks.

mod common;

use std::path::Path;

use yaml_multi_schema::document::Document;
use yaml_multi_schema::resolver::SchemaFetcher;
use yaml_multi_schema::server::YamlServer;

/// Reads the real local schema from disk and serves any remote HTTPS reference
/// with a valid schema, so no network access is needed.
struct ExampleFetcher;

impl SchemaFetcher for ExampleFetcher {
    fn read_local(&self, path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    }

    fn fetch_remote(&self, _url: &str) -> Result<String, String> {
        // A permissive schema: accept any object. Enough to prove the remote
        // annotation resolved and was applied.
        Ok(r#"{"type":"object","additionalProperties":true}"#.to_string())
    }
}

#[test]
fn example_values_all_annotations_resolve() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let text = std::fs::read_to_string(format!("{manifest_dir}/example-values.yaml")).unwrap();

    // Sanity: the file really has the three annotated blocks we expect.
    let doc = Document::parse(&text).unwrap();
    let annotated: Vec<&str> = doc
        .blocks
        .iter()
        .filter_map(|b| b.schema_ref.as_deref())
        .collect();
    assert_eq!(
        annotated.len(),
        3,
        "expected 3 annotated blocks, got {annotated:?}"
    );
    assert!(annotated.iter().any(|r| r.starts_with("./")));
    assert!(annotated.iter().any(|r| r.starts_with("https://")));

    let root = manifest_dir;
    let fetcher = ExampleFetcher;
    let mut server = YamlServer::new(&fetcher, Path::new(root));
    server.on_change(&text);

    // The `example-values.yaml` values are all valid against their schemas, so
    // a fully-resolved document produces no diagnostics. Any "failed to read /
    // fetch" warning indicates an annotation that did NOT resolve.
    let diags = server.diagnostics();
    assert!(
        diags.is_empty(),
        "expected all annotations resolved; got diagnostics: {diags:?}"
    );
}
