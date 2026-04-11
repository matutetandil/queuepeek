pub fn interpolate_template(body: &str, counter: &mut u64) -> String {
    let mut result = body.to_string();

    if result.contains("{{counter}}") {
        *counter += 1;
        result = result.replace("{{counter}}", &counter.to_string());
    }
    if result.contains("{{timestamp}}") {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        result = result.replace("{{timestamp}}", &ts.to_string());
    }
    if result.contains("{{uuid}}") {
        result = result.replace("{{uuid}}", &uuid::Uuid::new_v4().to_string());
    }
    if result.contains("{{random_int}}") {
        let r = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        result = result.replace("{{random_int}}", &(r % 1_000_000).to_string());
    }

    // Handle {{env.VAR}} patterns
    while let Some(start) = result.find("{{env.") {
        if let Some(end) = result[start..].find("}}") {
            let var_name = &result[start + 6..start + end];
            let val = std::env::var(var_name).unwrap_or_default();
            result = format!("{}{}{}", &result[..start], val, &result[start + end + 2..]);
        } else {
            break;
        }
    }

    result
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text.to_string()))
        .map_err(|e| format!("Clipboard: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_counter() {
        let mut counter = 0u64;
        let result = interpolate_template("msg-{{counter}}", &mut counter);
        assert_eq!(result, "msg-1");
        assert_eq!(counter, 1);

        let result2 = interpolate_template("msg-{{counter}}", &mut counter);
        assert_eq!(result2, "msg-2");
    }

    #[test]
    fn interpolate_timestamp() {
        let mut counter = 0u64;
        let result = interpolate_template("ts={{timestamp}}", &mut counter);
        assert!(result.starts_with("ts="));
        let ts: u64 = result[3..].parse().unwrap();
        assert!(ts > 1_000_000_000);
    }

    #[test]
    fn interpolate_uuid() {
        let mut counter = 0u64;
        let result = interpolate_template("id={{uuid}}", &mut counter);
        assert!(result.starts_with("id="));
        let uuid_part = &result[3..];
        assert_eq!(uuid_part.len(), 36); // UUID v4 format
        assert_eq!(uuid_part.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn interpolate_random_int() {
        let mut counter = 0u64;
        let result = interpolate_template("r={{random_int}}", &mut counter);
        assert!(result.starts_with("r="));
        let num: u32 = result[2..].parse().unwrap();
        assert!(num < 1_000_000);
    }

    #[test]
    fn interpolate_env_var() {
        std::env::set_var("QUEUEPEEK_TEST_VAR", "hello123");
        let mut counter = 0u64;
        let result = interpolate_template("val={{env.QUEUEPEEK_TEST_VAR}}", &mut counter);
        assert_eq!(result, "val=hello123");
        std::env::remove_var("QUEUEPEEK_TEST_VAR");
    }

    #[test]
    fn interpolate_missing_env_var() {
        let mut counter = 0u64;
        let result = interpolate_template("val={{env.NONEXISTENT_VAR_XYZ}}", &mut counter);
        assert_eq!(result, "val=");
    }

    #[test]
    fn interpolate_no_variables() {
        let mut counter = 0u64;
        let result = interpolate_template("plain text", &mut counter);
        assert_eq!(result, "plain text");
        assert_eq!(counter, 0);
    }

    #[test]
    fn interpolate_multiple_variables() {
        let mut counter = 0u64;
        let result = interpolate_template("{{counter}}-{{counter}}", &mut counter);
        // counter is incremented once for the first replacement, then both occurrences are replaced
        assert_eq!(result, "1-1");
        assert_eq!(counter, 1);
    }
}
