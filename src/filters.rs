use crate::backend::MessageInfo;

/// Filter expression for advanced message filtering
#[derive(Debug)]
pub enum FilterExpr {
    Substring(String),
    FieldEquals { field: String, value: String },
    FieldContains { field: String, value: String },
    FieldNotEquals { field: String, value: String },
}

/// Parse a filter expression string into a FilterExpr.
/// Supported syntax:
///   header.key = "value"    — exact match on header
///   body contains "text"    — substring match in body
///   routing_key = "value"   — exact match on routing_key
///   body.field = "value"    — JSON field match in body
///   field != "value"        — not-equals
pub fn parse_filter_expr(input: &str) -> FilterExpr {
    let input = input.trim();

    // Try "field contains value"
    if let Some(idx) = input.to_lowercase().find(" contains ") {
        let field = input[..idx].trim().to_string();
        let value = input[idx + 10..].trim().trim_matches('"').to_string();
        return FilterExpr::FieldContains { field, value };
    }

    // Try "field != value"
    if let Some(idx) = input.find("!=") {
        let field = input[..idx].trim().to_string();
        let value = input[idx + 2..].trim().trim_matches('"').to_string();
        return FilterExpr::FieldNotEquals { field, value };
    }

    // Try "field = value"
    if let Some(idx) = input.find('=') {
        let field = input[..idx].trim().to_string();
        let value = input[idx + 1..].trim().trim_matches('"').to_string();
        return FilterExpr::FieldEquals { field, value };
    }

    // Fallback to substring
    FilterExpr::Substring(input.to_lowercase())
}

/// Resolve a field path to a value from a MessageInfo
pub fn resolve_field(field: &str, msg: &MessageInfo) -> String {
    match field {
        "body" => msg.body.clone(),
        "routing_key" => msg.routing_key.clone(),
        "exchange" => msg.exchange.clone(),
        "content_type" => msg.content_type.clone(),
        "redelivered" => msg.redelivered.to_string(),
        _ if field.starts_with("header.") || field.starts_with("headers.") => {
            let key = field.splitn(2, '.').nth(1).unwrap_or("");
            msg.headers.iter()
                .find(|(k, _)| k.to_lowercase() == key.to_lowercase())
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        }
        _ if field.starts_with("body.") => {
            // JSON path lookup in body
            let path = field.splitn(2, '.').nth(1).unwrap_or("");
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg.body) {
                let parts: Vec<&str> = path.split('.').collect();
                let mut current = &val;
                for part in &parts {
                    if let Some(next) = current.get(part) {
                        current = next;
                    } else {
                        return String::new();
                    }
                }
                match current {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

pub fn eval_filter_expr(expr: &FilterExpr, msg: &MessageInfo) -> bool {
    match expr {
        FilterExpr::Substring(s) => {
            msg.body.to_lowercase().contains(s)
                || msg.routing_key.to_lowercase().contains(s)
        }
        FilterExpr::FieldEquals { field, value } => {
            let resolved = resolve_field(field, msg);
            resolved.to_lowercase() == value.to_lowercase()
        }
        FilterExpr::FieldContains { field, value } => {
            let resolved = resolve_field(field, msg);
            resolved.to_lowercase().contains(&value.to_lowercase())
        }
        FilterExpr::FieldNotEquals { field, value } => {
            let resolved = resolve_field(field, msg);
            resolved.to_lowercase() != value.to_lowercase()
        }
    }
}
