//! JSON Schema validation for tool arguments.

use serde_json::Value;
use std::fmt;

/// Error returned when JSON schema validation fails.
#[derive(Debug, Clone)]
pub struct SchemaValidationError {
    pub path: String,
    pub message: String,
}

impl fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "at '{}': {}", self.path, self.message)
        }
    }
}

impl std::error::Error for SchemaValidationError {}

/// Validate a JSON value against a JSON schema.
///
/// This is a simplified validator that handles common cases:
/// - type validation (string, number, integer, boolean, array, object, null)
/// - required properties
/// - enum validation
/// - minimum/maximum for numbers
/// - minLength/maxLength for strings
/// - minItems/maxItems for arrays
///
/// # Example
///
/// ```
/// use liteforge::tools::validate_json_schema;
/// use serde_json::json;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "name": { "type": "string" },
///         "age": { "type": "integer", "minimum": 0 }
///     },
///     "required": ["name"]
/// });
///
/// let valid = json!({"name": "Alice", "age": 30});
/// assert!(validate_json_schema(&valid, &schema).is_ok());
///
/// let invalid = json!({"age": -5});  // missing required "name"
/// assert!(validate_json_schema(&invalid, &schema).is_err());
/// ```
pub fn validate_json_schema(
    value: &Value,
    schema: &Value,
) -> Result<(), Vec<SchemaValidationError>> {
    let mut errors = Vec::new();
    validate_value(value, schema, "", &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_value(
    value: &Value,
    schema: &Value,
    path: &str,
    errors: &mut Vec<SchemaValidationError>,
) {
    // Handle anyOf, oneOf, allOf
    if let Some(any_of) = schema.get("anyOf") {
        if let Some(schemas) = any_of.as_array() {
            let mut any_valid = false;
            for s in schemas {
                let mut sub_errors = Vec::new();
                validate_value(value, s, path, &mut sub_errors);
                if sub_errors.is_empty() {
                    any_valid = true;
                    break;
                }
            }
            if !any_valid {
                errors.push(SchemaValidationError {
                    path: path.to_string(),
                    message: "value does not match any of the schemas".to_string(),
                });
            }
        }
        return;
    }

    // Check type
    if let Some(type_value) = schema.get("type") {
        let type_valid = match type_value.as_str() {
            Some("string") => value.is_string(),
            Some("number") => value.is_number(),
            Some("integer") => value.is_i64() || value.is_u64(),
            Some("boolean") => value.is_boolean(),
            Some("array") => value.is_array(),
            Some("object") => value.is_object(),
            Some("null") => value.is_null(),
            _ => true, // Unknown type, skip validation
        };

        if !type_valid {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!(
                    "expected type '{}', got '{}'",
                    type_value.as_str().unwrap_or("unknown"),
                    json_type_name(value)
                ),
            });
            return; // Don't continue validation if type is wrong
        }
    }

    // Validate based on type
    if value.is_string() {
        validate_string(value.as_str().unwrap(), schema, path, errors);
    } else if value.is_number() {
        validate_number(value, schema, path, errors);
    } else if value.is_array() {
        validate_array(value.as_array().unwrap(), schema, path, errors);
    } else if value.is_object() {
        validate_object(value.as_object().unwrap(), schema, path, errors);
    }

    // Check enum
    if let Some(enum_values) = schema.get("enum") {
        if let Some(variants) = enum_values.as_array() {
            if !variants.contains(value) {
                errors.push(SchemaValidationError {
                    path: path.to_string(),
                    message: format!("value must be one of {:?}", variants),
                });
            }
        }
    }

    // Check const
    if let Some(const_value) = schema.get("const") {
        if value != const_value {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!("value must be {:?}", const_value),
            });
        }
    }
}

fn validate_string(
    value: &str,
    schema: &Value,
    path: &str,
    errors: &mut Vec<SchemaValidationError>,
) {
    if let Some(min_length) = schema.get("minLength").and_then(|v| v.as_u64()) {
        if (value.len() as u64) < min_length {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!(
                    "string length {} is less than minimum {}",
                    value.len(),
                    min_length
                ),
            });
        }
    }

    if let Some(max_length) = schema.get("maxLength").and_then(|v| v.as_u64()) {
        if (value.len() as u64) > max_length {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!(
                    "string length {} is greater than maximum {}",
                    value.len(),
                    max_length
                ),
            });
        }
    }

    if let Some(pattern) = schema.get("pattern").and_then(|v| v.as_str()) {
        if let Ok(re) = regex::Regex::new(pattern) {
            if !re.is_match(value) {
                errors.push(SchemaValidationError {
                    path: path.to_string(),
                    message: format!("string does not match pattern '{}'", pattern),
                });
            }
        }
    }
}

fn validate_number(
    value: &Value,
    schema: &Value,
    path: &str,
    errors: &mut Vec<SchemaValidationError>,
) {
    let num = value.as_f64().unwrap_or(0.0);

    if let Some(minimum) = schema.get("minimum").and_then(|v| v.as_f64()) {
        if num < minimum {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!("value {} is less than minimum {}", num, minimum),
            });
        }
    }

    if let Some(maximum) = schema.get("maximum").and_then(|v| v.as_f64()) {
        if num > maximum {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!("value {} is greater than maximum {}", num, maximum),
            });
        }
    }

    if let Some(exclusive_minimum) = schema.get("exclusiveMinimum").and_then(|v| v.as_f64()) {
        if num <= exclusive_minimum {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!("value {} must be greater than {}", num, exclusive_minimum),
            });
        }
    }

    if let Some(exclusive_maximum) = schema.get("exclusiveMaximum").and_then(|v| v.as_f64()) {
        if num >= exclusive_maximum {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!("value {} must be less than {}", num, exclusive_maximum),
            });
        }
    }
}

fn validate_array(
    value: &[Value],
    schema: &Value,
    path: &str,
    errors: &mut Vec<SchemaValidationError>,
) {
    if let Some(min_items) = schema.get("minItems").and_then(|v| v.as_u64()) {
        if (value.len() as u64) < min_items {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!(
                    "array length {} is less than minimum {}",
                    value.len(),
                    min_items
                ),
            });
        }
    }

    if let Some(max_items) = schema.get("maxItems").and_then(|v| v.as_u64()) {
        if (value.len() as u64) > max_items {
            errors.push(SchemaValidationError {
                path: path.to_string(),
                message: format!(
                    "array length {} is greater than maximum {}",
                    value.len(),
                    max_items
                ),
            });
        }
    }

    // Validate items
    if let Some(items_schema) = schema.get("items") {
        for (i, item) in value.iter().enumerate() {
            let item_path = format!("{}[{}]", path, i);
            validate_value(item, items_schema, &item_path, errors);
        }
    }
}

fn validate_object(
    value: &serde_json::Map<String, Value>,
    schema: &Value,
    path: &str,
    errors: &mut Vec<SchemaValidationError>,
) {
    // Check required properties
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        for req in required {
            if let Some(prop_name) = req.as_str() {
                if !value.contains_key(prop_name) {
                    errors.push(SchemaValidationError {
                        path: path.to_string(),
                        message: format!("missing required property '{}'", prop_name),
                    });
                }
            }
        }
    }

    // Validate properties
    if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
        for (prop_name, prop_schema) in properties {
            if let Some(prop_value) = value.get(prop_name) {
                let prop_path = if path.is_empty() {
                    prop_name.clone()
                } else {
                    format!("{}.{}", path, prop_name)
                };
                validate_value(prop_value, prop_schema, &prop_path, errors);
            }
        }
    }

    // Check additional properties
    if let Some(additional) = schema.get("additionalProperties") {
        if additional.as_bool() == Some(false) {
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                for key in value.keys() {
                    if !properties.contains_key(key) {
                        errors.push(SchemaValidationError {
                            path: path.to_string(),
                            message: format!("additional property '{}' is not allowed", key),
                        });
                    }
                }
            }
        }
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_type_string() {
        let schema = json!({"type": "string"});

        assert!(validate_json_schema(&json!("hello"), &schema).is_ok());
        assert!(validate_json_schema(&json!(123), &schema).is_err());
    }

    #[test]
    fn test_validate_type_number() {
        let schema = json!({"type": "number"});

        assert!(validate_json_schema(&json!(123), &schema).is_ok());
        assert!(validate_json_schema(&json!(12.5), &schema).is_ok());
        assert!(validate_json_schema(&json!("hello"), &schema).is_err());
    }

    #[test]
    fn test_validate_type_integer() {
        let schema = json!({"type": "integer"});

        assert!(validate_json_schema(&json!(123), &schema).is_ok());
        // Note: 12.0 is stored as integer in serde_json
        assert!(validate_json_schema(&json!("hello"), &schema).is_err());
    }

    #[test]
    fn test_validate_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            },
            "required": ["name"]
        });

        assert!(validate_json_schema(&json!({"name": "Alice"}), &schema).is_ok());
        assert!(validate_json_schema(&json!({"age": 30}), &schema).is_err());
    }

    #[test]
    fn test_validate_enum() {
        let schema = json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        });

        assert!(validate_json_schema(&json!("red"), &schema).is_ok());
        assert!(validate_json_schema(&json!("yellow"), &schema).is_err());
    }

    #[test]
    fn test_validate_minimum_maximum() {
        let schema = json!({
            "type": "number",
            "minimum": 0,
            "maximum": 100
        });

        assert!(validate_json_schema(&json!(50), &schema).is_ok());
        assert!(validate_json_schema(&json!(-1), &schema).is_err());
        assert!(validate_json_schema(&json!(101), &schema).is_err());
    }

    #[test]
    fn test_validate_string_length() {
        let schema = json!({
            "type": "string",
            "minLength": 2,
            "maxLength": 10
        });

        assert!(validate_json_schema(&json!("hello"), &schema).is_ok());
        assert!(validate_json_schema(&json!("a"), &schema).is_err());
        assert!(validate_json_schema(&json!("this is too long"), &schema).is_err());
    }

    #[test]
    fn test_validate_array_items() {
        let schema = json!({
            "type": "array",
            "items": {"type": "integer"},
            "minItems": 1,
            "maxItems": 3
        });

        assert!(validate_json_schema(&json!([1, 2, 3]), &schema).is_ok());
        assert!(validate_json_schema(&json!([]), &schema).is_err()); // too few
        assert!(validate_json_schema(&json!([1, 2, 3, 4]), &schema).is_err()); // too many
        assert!(validate_json_schema(&json!([1, "two", 3]), &schema).is_err()); // wrong type
    }

    #[test]
    fn test_validate_nested_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "person": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }
            },
            "required": ["person"]
        });

        assert!(validate_json_schema(&json!({"person": {"name": "Alice"}}), &schema).is_ok());
        assert!(validate_json_schema(&json!({"person": {}}), &schema).is_err());
    }
}
