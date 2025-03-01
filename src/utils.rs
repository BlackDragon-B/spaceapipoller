use serde_json::from_str;
use spaceapi::Status;

pub(crate) fn get_endpoint(endpoint: &str) -> Result<Status, String> {
    match reqwest::blocking::Client::new().get(endpoint).send() {
        Ok(result) => {
            let state: Result<Status, serde_json::Error> = from_str(&result.text().unwrap());
            match state {
                Ok(state) => Ok(state),
                Err(err) => Err(format!("Failed to parse endpoint response: {}",err.to_string())),
            }
        }, 
        Err(err) => Err(format!("Endpoint unavailable: {}",err.to_string())),
    }
}

pub(crate) fn to_safe_entity_name(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    // Ensure the first character is a lowercase letter
    if let Some(&first_char) = chars.peek() {
        if first_char.is_ascii_alphabetic() {
            result.push(first_char.to_ascii_lowercase());
            chars.next();
        } else {
            result.push('_');
        }
    }

    // Process the remaining characters
    for c in chars {
        if c.is_ascii_alphanumeric() {
            result.push(c.to_ascii_lowercase());
        } else {
            result.push('_');
        }
    }

    // Replace multiple underscores with a single one
    let mut final_result = String::new();
    let mut prev_char = '\0';
    for c in result.chars() {
        if c != '_' || prev_char != '_' {
            final_result.push(c);
        }
        prev_char = c;
    }

    final_result
}
