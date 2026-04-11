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
