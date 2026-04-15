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
            let key = field.split_once('.').map(|x| x.1).unwrap_or("");
            msg.headers.iter()
                .find(|(k, _)| k.to_lowercase() == key.to_lowercase())
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        }
        _ if field.starts_with("body.") => {
            // JSON path lookup in body
            let path = field.split_once('.').map(|x| x.1).unwrap_or("");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MessageInfo;

    fn make_msg(body: &str, routing_key: &str, headers: Vec<(&str, &str)>) -> MessageInfo {
        MessageInfo {
            index: 1,
            routing_key: routing_key.to_string(),
            exchange: "test-exchange".to_string(),
            redelivered: false,
            timestamp: None,
            content_type: "application/json".to_string(),
            headers: headers.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            body: body.to_string(),
        }
    }

    // --- parse_filter_expr ---

    #[test]
    fn parse_substring() {
        let expr = parse_filter_expr("hello");
        assert!(matches!(expr, FilterExpr::Substring(s) if s == "hello"));
    }

    #[test]
    fn parse_field_equals() {
        let expr = parse_filter_expr("routing_key = \"orders\"");
        assert!(matches!(expr, FilterExpr::FieldEquals { ref field, ref value } if field == "routing_key" && value == "orders"));
    }

    #[test]
    fn parse_field_contains() {
        let expr = parse_filter_expr("body contains \"error\"");
        assert!(matches!(expr, FilterExpr::FieldContains { ref field, ref value } if field == "body" && value == "error"));
    }

    #[test]
    fn parse_field_not_equals() {
        let expr = parse_filter_expr("exchange != \"dlx\"");
        assert!(matches!(expr, FilterExpr::FieldNotEquals { ref field, ref value } if field == "exchange" && value == "dlx"));
    }

    #[test]
    fn parse_contains_case_insensitive() {
        let expr = parse_filter_expr("body CONTAINS \"Test\"");
        assert!(matches!(expr, FilterExpr::FieldContains { .. }));
    }

    // --- resolve_field ---

    #[test]
    fn resolve_body() {
        let msg = make_msg("hello world", "key", vec![]);
        assert_eq!(resolve_field("body", &msg), "hello world");
    }

    #[test]
    fn resolve_routing_key() {
        let msg = make_msg("", "my.key", vec![]);
        assert_eq!(resolve_field("routing_key", &msg), "my.key");
    }

    #[test]
    fn resolve_header() {
        let msg = make_msg("", "", vec![("x-type", "important")]);
        assert_eq!(resolve_field("header.x-type", &msg), "important");
    }

    #[test]
    fn resolve_header_case_insensitive() {
        let msg = make_msg("", "", vec![("Content-Type", "text/plain")]);
        assert_eq!(resolve_field("header.content-type", &msg), "text/plain");
    }

    #[test]
    fn resolve_body_json_path() {
        let msg = make_msg(r#"{"user": {"id": 42, "name": "alice"}}"#, "", vec![]);
        assert_eq!(resolve_field("body.user.name", &msg), "alice");
        assert_eq!(resolve_field("body.user.id", &msg), "42");
    }

    #[test]
    fn resolve_body_json_path_missing() {
        let msg = make_msg(r#"{"foo": "bar"}"#, "", vec![]);
        assert_eq!(resolve_field("body.nonexistent.path", &msg), "");
    }

    #[test]
    fn resolve_unknown_field() {
        let msg = make_msg("", "", vec![]);
        assert_eq!(resolve_field("unknown_field", &msg), "");
    }

    // --- eval_filter_expr ---

    #[test]
    fn eval_substring_matches_body() {
        let msg = make_msg("hello world", "", vec![]);
        let expr = parse_filter_expr("hello");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_substring_matches_routing_key() {
        let msg = make_msg("", "order.created", vec![]);
        let expr = parse_filter_expr("order");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_substring_no_match() {
        let msg = make_msg("foo", "bar", vec![]);
        let expr = parse_filter_expr("baz");
        assert!(!eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_field_equals_match() {
        let msg = make_msg("", "orders", vec![]);
        let expr = parse_filter_expr("routing_key = \"orders\"");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_field_equals_case_insensitive() {
        let msg = make_msg("", "Orders", vec![]);
        let expr = parse_filter_expr("routing_key = \"orders\"");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_field_contains() {
        let msg = make_msg(r#"{"error": "something failed"}"#, "", vec![]);
        let expr = parse_filter_expr("body contains \"failed\"");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_field_not_equals() {
        let msg = make_msg("", "", vec![]);
        let expr = parse_filter_expr("exchange != \"dlx\"");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_header_field_equals() {
        let msg = make_msg("", "", vec![("x-type", "important")]);
        let expr = parse_filter_expr("header.x-type = \"important\"");
        assert!(eval_filter_expr(&expr, &msg));
    }

    #[test]
    fn eval_json_path_equals() {
        let msg = make_msg(r#"{"status": "error"}"#, "", vec![]);
        let expr = parse_filter_expr("body.status = \"error\"");
        assert!(eval_filter_expr(&expr, &msg));
    }
}
