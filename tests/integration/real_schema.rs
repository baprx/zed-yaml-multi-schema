//! Regression test: validates against the real bundled `schemas/test.schema.json`
//! (a draft-07 schema). Guards against the validator ignoring keywords when a
//! schema declares a non-2020-12 `$schema`.

struct NoopFetcher;
impl zed_yaml_multi_schema::resolver::SchemaFetcher for NoopFetcher {
    fn read_local(&self, _path: &str) -> Result<String, String> {
        Err("n/a".into())
    }
    fn fetch_remote(&self, _url: &str) -> Result<String, String> {
        Err("n/a".into())
    }
}

#[test]
fn real_draft07_schema_still_validates() {
    let text = std::fs::read_to_string("schemas/test.schema.json").unwrap();
    let schema: serde_json::Value = serde_json::from_str(&text).unwrap();

    let value: yaml_serde::Value =
        yaml_serde::from_str("enabled: not-a-bool\nelements: [1]\nproduct: anch\nversion: 1\n")
            .unwrap();
    let findings = zed_yaml_multi_schema::validator::validate(
        &schema,
        &value,
        std::sync::Arc::new(NoopFetcher),
    )
    .unwrap();
    assert!(
        !findings.is_empty(),
        "draft-07 schema did not reject a string for a boolean"
    );

    let ok: yaml_serde::Value =
        yaml_serde::from_str("enabled: true\nelements: [1, A]\nproduct: anch\nversion: 1\n")
            .unwrap();
    let findings =
        zed_yaml_multi_schema::validator::validate(&schema, &ok, std::sync::Arc::new(NoopFetcher))
            .unwrap();
    assert!(findings.is_empty(), "valid values should pass");
}
