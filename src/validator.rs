//! JSON Schema 2020-12 validation of a block value against a resolved schema.

use std::sync::Arc;

use crate::resolver::SchemaFetcher;
use jsonschema::{Retrieve, Uri, Validator};
use serde_json::Value;

/// A single validation finding against a governed block.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Path to the offending value, e.g. `/enabled`.
    pub instance_path: String,
    /// Human-readable message.
    pub message: String,
}

/// Adapts our `SchemaFetcher` to jsonschema's `Retrieve` trait.
///
/// jsonschema calls this whenever it hits a `$ref` inside a schema that
/// points to a URI not already in its registry — e.g. the nested
/// `definitions.json` ref inside bjw-s-labs' `values.schema.json`.
/// Without it, jsonschema has no way to fetch that ref itself, since
/// `resolve-http`/`resolve-file` are compiled out (default-features = false
/// in Cargo.toml).
pub(crate) struct FetcherRetriever {
    pub(crate) fetcher: Arc<dyn SchemaFetcher>,
}

/// Fetches and parses an external schema reference on jsonschema's behalf.
///
/// Delegates to the same `fetch_remote()` that `SchemaResolver` already
/// uses for the top-level `# $schema=` reference, so both resolution paths
/// go through one fetch implementation.
impl Retrieve for FetcherRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let t0 = std::time::Instant::now();
        let text = self
            .fetcher
            .fetch_remote(uri.as_str())
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;
        eprintln!(
            "yaml-multi-schema-lsp: retrieve {} took {:?}",
            uri.as_str(),
            t0.elapsed()
        );
        serde_json::from_str(&text)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })
    }
}

/// Validates `value` against a parsed JSON Schema.
///
/// The draft is detected from the schema's `$schema` keyword (defaulting to the
/// crate default), so schemas declaring 2020-12 are honored while draft-07
/// schemas (common in Helm charts) validate correctly. Unknown keywords degrade
/// gracefully rather than fail.
pub fn validate(validator: &Validator, value: &yaml_serde::Value) -> Result<Vec<Finding>, String> {
    let instance = to_json(value);
    let mut findings = Vec::new();
    for error in validator.iter_errors(&instance) {
        findings.push(Finding {
            instance_path: error.instance_path().to_string(),
            message: error.to_string(),
        });
    }
    Ok(findings)
}

/// Converts a yaml_serde value to serde_json for validation.
fn to_json(value: &yaml_serde::Value) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reports_violation_for_invalid_value() {
        let schema = json!({"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"]});
        let validator = Validator::options().build(&schema).unwrap();
        let value: yaml_serde::Value = yaml_serde::from_str("enabled: not-a-bool\n").unwrap();
        let findings = validate(&validator, &value).unwrap();
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.instance_path.contains("enabled")));
    }

    #[test]
    fn passes_valid_value() {
        let schema = json!({"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"]});
        let validator = Validator::options().build(&schema).unwrap();
        let value: yaml_serde::Value = yaml_serde::from_str("enabled: true\n").unwrap();
        let findings = validate(&validator, &value).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_unknown_keywords() {
        let schema =
            json!({"type":"object","properties":{"a":{"type":"integer"}},"x-custom-unknown":42});
        let validator = Validator::options().build(&schema).unwrap();
        let value: yaml_serde::Value = yaml_serde::from_str("a: 1\n").unwrap();
        let findings = validate(&validator, &value).unwrap();
        assert!(findings.is_empty());
    }
}
