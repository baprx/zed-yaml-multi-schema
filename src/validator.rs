//! JSON Schema 2020-12 validation of a block value against a resolved schema.

use jsonschema::Validator;

/// A single validation finding against a governed block.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Path to the offending value, e.g. `/enabled`.
    pub instance_path: String,
    /// Human-readable message.
    pub message: String,
}

/// Validates `value` against a parsed JSON Schema.
///
/// The draft is detected from the schema's `$schema` keyword (defaulting to the
/// crate default), so schemas declaring 2020-12 are honored while draft-07
/// schemas (common in Helm charts) validate correctly. Unknown keywords degrade
/// gracefully rather than fail.
pub fn validate(
    schema: &serde_json::Value,
    value: &serde_yaml::Value,
) -> Result<Vec<Finding>, String> {
    let validator = Validator::options()
        .build(schema)
        .map_err(|e| format!("invalid schema: {e}"))?;

    let instance = to_json(value);
    let mut findings = Vec::new();
    for error in validator.iter_errors(&instance) {
        findings.push(Finding {
            instance_path: error.instance_path.to_string(),
            message: error.to_string(),
        });
    }
    Ok(findings)
}

/// Converts a serde_yaml value to serde_json for validation.
fn to_json(value: &serde_yaml::Value) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reports_violation_for_invalid_value() {
        let schema = json!({"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"]});
        let value: serde_yaml::Value = serde_yaml::from_str("enabled: not-a-bool\n").unwrap();
        let findings = validate(&schema, &value).unwrap();
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.instance_path.contains("enabled")));
    }

    #[test]
    fn passes_valid_value() {
        let schema = json!({"type":"object","properties":{"enabled":{"type":"boolean"}},"required":["enabled"]});
        let value: serde_yaml::Value = serde_yaml::from_str("enabled: true\n").unwrap();
        let findings = validate(&schema, &value).unwrap();
        assert!(findings.is_empty());
    }

    #[test]
    fn ignores_unknown_keywords() {
        let schema =
            json!({"type":"object","properties":{"a":{"type":"integer"}},"x-custom-unknown":42});
        let value: serde_yaml::Value = serde_yaml::from_str("a: 1\n").unwrap();
        let findings = validate(&schema, &value).unwrap();
        assert!(findings.is_empty());
    }
}
