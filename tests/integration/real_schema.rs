//! Regression test: validates against the real bundled `schemas/test.schema.json`
//! (a draft-07 schema). Guards against the validator ignoring keywords when a
//! schema declares a non-2020-12 `$schema`.

use jsonschema::Validator;

#[test]
fn real_draft07_schema_still_validates() {
    let text = std::fs::read_to_string("schemas/test.schema.json").unwrap();
    let schema: serde_json::Value = serde_json::from_str(&text).unwrap();
    let validator = Validator::options().build(&schema).unwrap();

    let value: yaml_serde::Value =
        yaml_serde::from_str("enabled: not-a-bool\nelements: [1]\nproduct: anch\nversion: 1\n")
            .unwrap();
    let findings = zed_yaml_multi_schema::validator::validate(&validator, &value).unwrap();
    assert!(
        !findings.is_empty(),
        "draft-07 schema did not reject a string for a boolean"
    );

    let ok: yaml_serde::Value =
        yaml_serde::from_str("enabled: true\nelements: [1, A]\nproduct: anch\nversion: 1\n")
            .unwrap();
    let findings = zed_yaml_multi_schema::validator::validate(&validator, &ok).unwrap();
    assert!(findings.is_empty(), "valid values should pass");
}
