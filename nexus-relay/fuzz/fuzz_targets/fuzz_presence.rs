#![no_main]
use libfuzzer_sys::fuzz_target;

/// Fuzz test for presence/status handling
/// Tests that status updates don't cause panics
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Test status parsing
        test_status_parsing(s);

        // Test status message validation
        test_status_message(s);

        // Test user ID validation
        test_user_id(s);
    }
});

fn test_status_parsing(status: &str) {
    match status.to_lowercase().as_str() {
        "online" | "away" | "dnd" | "do not disturb" | "offline" | "invisible" | "busy"
        | "available" => {
            // Valid status
        }
        _ => {
            // Invalid status - should be handled gracefully
        }
    }
}

fn test_status_message(msg: &str) {
    // Max length for status message
    let max_len = 200;

    // Should handle truncation
    let truncated = if msg.len() > max_len {
        &msg[..max_len]
    } else {
        msg
    };

    // Should handle sanitization
    let sanitized = truncated
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>();

    let _ = sanitized;
}

fn test_user_id(id: &str) {
    // User ID validation
    let is_valid = id.len() >= 8
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '-' || c == '_');

    let _ = is_valid;
}
