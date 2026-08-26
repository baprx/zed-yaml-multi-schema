//! Derive completion items from a schema governing a block.

use serde_json::Value;

/// A completion candidate derived from a schema.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    /// LSP `CompletionItemKind` (integer enum), e.g. 10 = Property.
    pub kind: i32,
    pub detail: Option<String>,
}

/// Returns property-name completions from `schema` at the given nesting depth.
/// For v1 this offers the top-level `properties` keys of the schema's object.
pub fn complete(schema: &Value, _depth: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return items;
    };
    for (name, prop) in props {
        let detail = prop
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);
        let kind = prop_type(prop);
        items.push(CompletionItem {
            label: name.clone(),
            kind,
            detail,
        });
    }
    items
}

fn prop_type(prop: &Value) -> i32 {
    let t = prop
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("property");
    match t {
        "object" => 9,   // Module
        "array" => 11,   // Unit
        "boolean" => 21, // Constant
        "number" => 12,  // Value
        "integer" => 12, // Value
        "null" => 21,    // Constant
        "string" => 10,  // Property
        _ => 10,         // Property
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lists_property_names() {
        let schema = json!({
            "type": "object",
            "properties": {
                "enabled": {"type": "boolean", "description": "Whether enabled"},
                "version": {"type": "number"}
            }
        });
        let items = complete(&schema, 0);
        let labels: Vec<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(labels.contains(&"enabled".to_string()));
        assert!(labels.contains(&"version".to_string()));
        let enabled = items.iter().find(|i| i.label == "enabled").unwrap();
        assert_eq!(enabled.detail.as_deref(), Some("Whether enabled"));
    }
}
