//! Tool definition and result types.

use serde::{Deserialize, Serialize};

/// Definition of a tool that an agent can use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool identifier.
    pub name: String,
    /// Human-readable description for the LLM.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool use instance.
    pub id: String,
    /// Which tool to call.
    pub name: String,
    /// The input parameters.
    pub input: serde_json::Value,
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool_use ID this result corresponds to.
    pub tool_use_id: String,
    /// The output content.
    pub content: String,
    /// Whether the tool execution resulted in an error.
    pub is_error: bool,
}

/// Providers that require `type` to be a single string (no array).
const TYPE_SINGLE_ONLY_PROVIDERS: &[&str] = &["groq", "openai"];

/// Normalize a JSON Schema for cross-provider compatibility.
///
/// Some providers (Gemini, Groq) reject `anyOf` in tool schemas.
/// Groq/OpenAI also reject `type: ["string", "integer"]` — only a single type is allowed.
/// This function:
/// - Converts `anyOf` arrays of simple types to flat type (single type for groq/openai)
/// - Strips `$schema` keys (not accepted by most providers)
/// - Recursively walks `properties` and `items`
/// - For groq/openai: ensures every `type` is a string, not an array
pub fn normalize_schema_for_provider(
    schema: &serde_json::Value,
    provider: &str,
) -> serde_json::Value {
    // Anthropic handles anyOf natively — no normalization needed
    if provider == "anthropic" {
        return schema.clone();
    }
    let type_single_only = TYPE_SINGLE_ONLY_PROVIDERS.contains(&provider);
    normalize_schema_recursive(schema, type_single_only)
}

/// Schema-structural keys that conflict with `anyOf` on strict providers (Groq/OpenAI).
/// When `anyOf` can't be flattened, these sibling keys cause validation errors.
const ANYOF_CONFLICTING_SIBLINGS: &[&str] =
    &["type", "items", "additionalProperties", "properties"];

fn normalize_schema_recursive(
    schema: &serde_json::Value,
    type_single_only: bool,
) -> serde_json::Value {
    let obj = match schema.as_object() {
        Some(o) => o,
        None => return schema.clone(),
    };

    // Check if anyOf is present but can't be flattened — we'll need to strip
    // conflicting sibling keys for strict providers.
    let has_unflattenable_anyof = if let Some(any_of) = obj.get("anyOf") {
        type_single_only && try_flatten_any_of(any_of, type_single_only).is_none()
    } else {
        false
    };

    let mut result = serde_json::Map::new();

    for (key, value) in obj {
        // Strip $schema and $ref keys (not accepted by most providers)
        if key == "$schema" || key == "$ref" {
            continue;
        }

        // Convert anyOf to flat type + enum when possible
        if key == "anyOf" {
            if let Some(converted) = try_flatten_any_of(value, type_single_only) {
                for (k, v) in converted {
                    result.insert(k, v);
                }
                continue;
            }
            // Can't flatten — normalize each variant schema and keep anyOf
            if let Some(arr) = value.as_array() {
                let normalized: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| normalize_schema_recursive(v, type_single_only))
                    .collect();
                result.insert(key.clone(), serde_json::Value::Array(normalized));
                continue;
            }
        }

        // Strip schema-structural siblings when anyOf couldn't be flattened
        // (Groq/Gemini reject anyOf with type/items/properties siblings)
        if has_unflattenable_anyof && ANYOF_CONFLICTING_SIBLINGS.contains(&key.as_str()) {
            continue;
        }

        // Recurse into properties
        if key == "properties" {
            if let Some(props) = value.as_object() {
                let mut new_props = serde_json::Map::new();
                for (prop_name, prop_schema) in props {
                    new_props.insert(
                        prop_name.clone(),
                        normalize_schema_recursive(prop_schema, type_single_only),
                    );
                }
                result.insert(key.clone(), serde_json::Value::Object(new_props));
                continue;
            }
        }

        // Recurse into items (object = single schema, array = tuple validation)
        if key == "items" {
            if value.is_array() {
                if let Some(arr) = value.as_array() {
                    let normalized: Vec<serde_json::Value> = arr
                        .iter()
                        .map(|v| normalize_schema_recursive(v, type_single_only))
                        .collect();
                    result.insert(key.clone(), serde_json::Value::Array(normalized));
                }
            } else {
                result.insert(
                    key.clone(),
                    normalize_schema_recursive(value, type_single_only),
                );
            }
            continue;
        }

        // Recurse into additionalProperties when it's a schema object
        if key == "additionalProperties" && value.is_object() {
            result.insert(
                key.clone(),
                normalize_schema_recursive(value, type_single_only),
            );
            continue;
        }

        // Recurse into composition keywords: allOf, oneOf
        if key == "allOf" || key == "oneOf" {
            if let Some(arr) = value.as_array() {
                let normalized: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| normalize_schema_recursive(v, type_single_only))
                    .collect();
                result.insert(key.clone(), serde_json::Value::Array(normalized));
                continue;
            }
        }

        // Recurse into not, if, then, else (single schema)
        if (key == "not" || key == "if" || key == "then" || key == "else") && value.is_object() {
            result.insert(
                key.clone(),
                normalize_schema_recursive(value, type_single_only),
            );
            continue;
        }

        // Recurse into prefixItems (array of schemas, JSON Schema 2020-12 tuple)
        if key == "prefixItems" {
            if let Some(arr) = value.as_array() {
                let normalized: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| normalize_schema_recursive(v, type_single_only))
                    .collect();
                result.insert(key.clone(), serde_json::Value::Array(normalized));
                continue;
            }
        }

        // Recurse into definitions / $defs (map of name → schema)
        if key == "definitions" || key == "$defs" {
            if let Some(defs) = value.as_object() {
                let mut new_defs = serde_json::Map::new();
                for (def_name, def_schema) in defs {
                    new_defs.insert(
                        def_name.clone(),
                        normalize_schema_recursive(def_schema, type_single_only),
                    );
                }
                result.insert(key.clone(), serde_json::Value::Object(new_defs));
                continue;
            }
        }

        // Recurse into patternProperties (map of pattern → schema)
        if key == "patternProperties" {
            if let Some(pp) = value.as_object() {
                let mut new_pp = serde_json::Map::new();
                for (pattern, pp_schema) in pp {
                    new_pp.insert(
                        pattern.clone(),
                        normalize_schema_recursive(pp_schema, type_single_only),
                    );
                }
                result.insert(key.clone(), serde_json::Value::Object(new_pp));
                continue;
            }
        }

        // Groq/OpenAI: type must be a single string, not an array
        if type_single_only && key == "type" && value.is_array() {
            let single = type_array_to_single(value);
            result.insert(key.clone(), single);
            continue;
        }

        result.insert(key.clone(), value.clone());
    }

    serde_json::Value::Object(result)
}

/// Coerce a JSON Schema type array to a single type string for strict validators (Groq/OpenAI).
fn type_array_to_single(arr: &serde_json::Value) -> serde_json::Value {
    let arr = match arr.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return serde_json::Value::String("string".to_string()),
    };
    let first = arr.iter().find_map(|v| v.as_str()).filter(|s| *s != "null");
    serde_json::Value::String(first.unwrap_or("string").to_string())
}

/// Try to flatten an `anyOf` array into a simple type + enum.
///
/// Works when all variants are simple types (string, number, etc.) or
/// when it's a nullable pattern like `anyOf: [{type: "string"}, {type: "null"}]`.
/// When type_single_only is true (Groq/OpenAI), multi-type anyOf becomes a single type (first).
fn try_flatten_any_of(
    any_of: &serde_json::Value,
    type_single_only: bool,
) -> Option<Vec<(String, serde_json::Value)>> {
    let items = any_of.as_array()?;
    if items.is_empty() {
        return None;
    }

    // Check if this is a simple type union (all items have just "type")
    let mut types = Vec::new();
    let mut has_null = false;
    let mut non_null_type = None;

    for item in items {
        let obj = item.as_object()?;
        let type_val = obj.get("type")?.as_str()?;

        // Only flatten simple type variants (those with just "type" and
        // optionally "enum"/"const"). If the variant has properties, items,
        // allOf, etc., it's too complex to flatten.
        let complex_keys = [
            "properties",
            "items",
            "allOf",
            "oneOf",
            "anyOf",
            "additionalProperties",
        ];
        if obj.keys().any(|k| complex_keys.contains(&k.as_str())) {
            return None;
        }

        if type_val == "null" {
            has_null = true;
        } else {
            types.push(type_val.to_string());
            non_null_type = Some(type_val.to_string());
        }
    }

    // If it's a nullable pattern (type + null), emit the non-null type
    if has_null && types.len() == 1 {
        let mut result = vec![(
            "type".to_string(),
            serde_json::Value::String(non_null_type.unwrap()),
        )];
        // Mark as nullable via description hint (since JSON Schema nullable isn't universal)
        result.push(("nullable".to_string(), serde_json::Value::Bool(true)));
        return Some(result);
    }

    // If all items are simple types: for Groq/OpenAI emit single type, else type array
    if types.len() == items.len() && types.len() > 1 {
        let type_value = if type_single_only {
            serde_json::Value::String(types.into_iter().next().unwrap_or_else(|| "string".into()))
        } else {
            serde_json::Value::Array(types.into_iter().map(serde_json::Value::String).collect())
        };
        return Some(vec![("type".to_string(), type_value)]);
    }

    // Can't flatten — leave as-is
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"]
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("web_search"));
    }

    #[test]
    fn test_normalize_schema_strips_dollar_schema() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });
        let result = normalize_schema_for_provider(&schema, "gemini");
        assert!(result.get("$schema").is_none());
        assert_eq!(result["type"], "object");
    }

    #[test]
    fn test_normalize_schema_flattens_anyof_nullable() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "value": {
                    "anyOf": [
                        { "type": "string" },
                        { "type": "null" }
                    ]
                }
            }
        });
        let result = normalize_schema_for_provider(&schema, "gemini");
        let value_prop = &result["properties"]["value"];
        assert_eq!(value_prop["type"], "string");
        assert_eq!(value_prop["nullable"], true);
        assert!(value_prop.get("anyOf").is_none());
    }

    #[test]
    fn test_normalize_schema_flattens_anyof_multi_type_groq_single_type() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "value": {
                    "anyOf": [
                        { "type": "string" },
                        { "type": "number" }
                    ]
                }
            }
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        let value_prop = &result["properties"]["value"];
        // Groq requires type to be a single string
        assert_eq!(value_prop["type"], "string");
    }

    #[test]
    fn test_normalize_schema_type_array_to_single_groq() {
        // MCP/OpenAPI can emit type: ["string", "null"] or type array in items
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "comments": {
                    "type": "array",
                    "items": { "type": ["string", "object"] }
                }
            }
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        let items = &result["properties"]["comments"]["items"];
        assert_eq!(items["type"], "string");
    }

    #[test]
    fn test_normalize_schema_anthropic_passthrough() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "anyOf": [{"type": "string"}]
        });
        let result = normalize_schema_for_provider(&schema, "anthropic");
        // Anthropic should get the original schema unchanged
        assert!(result.get("$schema").is_some());
    }

    #[test]
    fn test_normalize_schema_nested_properties() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "properties": {
                        "inner": {
                            "$schema": "strip_me",
                            "type": "string"
                        }
                    }
                }
            }
        });
        let result = normalize_schema_for_provider(&schema, "gemini");
        assert!(result["properties"]["outer"]["properties"]["inner"]
            .get("$schema")
            .is_none());
    }

    #[test]
    fn test_normalize_anyof_with_sibling_type_stripped_for_groq() {
        // MCP GitHub server pattern: anyOf with type/items siblings
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "comments": {
                    "type": "array",
                    "anyOf": [
                        {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "body": { "type": "string" }
                                }
                            }
                        }
                    ],
                    "items": {
                        "type": ["object", "null"]
                    },
                    "description": "Review comments"
                }
            }
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        let comments = &result["properties"]["comments"];
        // anyOf should be kept (can't flatten), but type/items siblings stripped
        assert!(comments.get("anyOf").is_some());
        assert!(
            comments.get("type").is_none(),
            "type sibling should be stripped when anyOf is present for groq"
        );
        assert!(
            comments.get("items").is_none(),
            "items sibling should be stripped when anyOf is present for groq"
        );
        // description is NOT a conflicting sibling — it should be kept
        assert_eq!(comments["description"], "Review comments");
    }

    #[test]
    fn test_normalize_recurses_into_allof() {
        let schema = serde_json::json!({
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "field": { "type": ["string", "null"] }
                    }
                }
            ]
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        let field = &result["allOf"][0]["properties"]["field"];
        assert_eq!(field["type"], "string");
    }

    #[test]
    fn test_normalize_recurses_into_oneof() {
        let schema = serde_json::json!({
            "oneOf": [
                { "type": ["string", "null"] },
                { "type": "integer" }
            ]
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        assert_eq!(result["oneOf"][0]["type"], "string");
        assert_eq!(result["oneOf"][1]["type"], "integer");
    }

    #[test]
    fn test_normalize_recurses_into_additional_properties() {
        let schema = serde_json::json!({
            "type": "object",
            "additionalProperties": {
                "type": ["string", "number"]
            }
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        assert_eq!(result["additionalProperties"]["type"], "string");
    }

    #[test]
    fn test_normalize_recurses_into_unflattenable_anyof_variants() {
        // anyOf with complex objects that can't be flattened
        let schema = serde_json::json!({
            "anyOf": [
                {
                    "type": "object",
                    "properties": {
                        "inner": { "type": ["string", "null"] }
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "other": { "$schema": "strip_me", "type": "number" }
                    }
                }
            ]
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        // anyOf should be preserved (both are objects, can be flattened to type:"object"
        // actually — but the variants' internals should be normalized)
        // The type arrays inside the variants should be normalized
        let inner = &result["anyOf"][0]["properties"]["inner"];
        assert_eq!(
            inner["type"], "string",
            "type array inside anyOf variant should be normalized"
        );
        let other = &result["anyOf"][1]["properties"]["other"];
        assert!(
            other.get("$schema").is_none(),
            "$schema inside anyOf variant should be stripped"
        );
    }

    #[test]
    fn test_normalize_items_as_array_tuple() {
        let schema = serde_json::json!({
            "type": "array",
            "items": [
                { "type": ["string", "null"] },
                { "type": "integer" }
            ]
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        assert_eq!(result["items"][0]["type"], "string");
        assert_eq!(result["items"][1]["type"], "integer");
    }

    #[test]
    fn test_normalize_strips_dollar_ref() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "field": { "$ref": "#/definitions/Foo", "type": "string" }
            }
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        assert!(result["properties"]["field"].get("$ref").is_none());
        assert_eq!(result["properties"]["field"]["type"], "string");
    }

    #[test]
    fn test_normalize_mcp_github_create_pull_request_review() {
        // Simulates the actual MCP GitHub schema pattern for create_pull_request_review
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "owner": { "type": "string" },
                "repo": { "type": "string" },
                "pull_number": { "type": "number" },
                "comments": {
                    "type": "array",
                    "anyOf": [
                        {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "body": { "type": "string" },
                                    "side": {
                                        "anyOf": [
                                            { "type": "string" },
                                            { "type": "null" }
                                        ]
                                    },
                                    "line": { "type": ["integer", "null"] }
                                },
                                "required": ["path", "body"]
                            }
                        }
                    ],
                    "description": "Review comments"
                }
            }
        });
        let result = normalize_schema_for_provider(&schema, "groq");
        let comments = &result["properties"]["comments"];
        // anyOf is unflattenable (single complex variant) — siblings stripped
        assert!(
            comments.get("type").is_none(),
            "type sibling should be stripped"
        );
        // Inside the anyOf variant, nested schemas should be normalized
        let items_inner = &comments["anyOf"][0]["items"];
        let side = &items_inner["properties"]["side"];
        // anyOf with nullable pattern should be flattened
        assert_eq!(side["type"], "string");
        // type array should be converted to single
        let line = &items_inner["properties"]["line"];
        assert_eq!(line["type"], "integer");
    }
}
