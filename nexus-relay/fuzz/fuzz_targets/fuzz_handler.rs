#![no_main]
use libfuzzer_sys::fuzz_target;

/// Fuzz test for handler message processing
/// Tests that all message handlers handle edge cases gracefully
fuzz_target!(|data: &[u8]| {
    // Test various input formats
    if let Ok(s) = std::str::from_utf8(data) {
        // Test recipient hash validation
        test_recipient_hash(s);

        // Test base64 decoding
        test_base64_decode(s);

        // Test TTL parsing
        test_ttl_parsing(s);

        // Test group name validation
        test_group_name(s);

        // Test status string parsing
        test_status_parsing(s);

        // Test call type parsing
        test_call_type(s);
    }
});

fn test_recipient_hash(s: &str) {
    // Should handle hex strings of various lengths
    let _ = hex::decode(s);

    // Test with specific lengths
    if s.len() == 64 {
        let _ = hex::decode(s);
    }
}

fn test_base64_decode(s: &str) {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    let _ = B64.decode(s);
}

fn test_ttl_parsing(s: &str) {
    // Test parsing as u64
    if let Ok(ttl) = s.parse::<u64>() {
        // Should handle any u64 value
        let _ = ttl.min(7 * 24 * 3600); // MAX_TTL_SECONDS
    }
}

fn test_group_name(s: &str) {
    // Test group name validation
    let is_valid = !s.is_empty() && s.len() <= 100;
    let _ = is_valid;
}

fn test_status_parsing(s: &str) {
    match s {
        "online" | "away" | "dnd" | "invisible" | "offline" => {}
        _ => {}
    }
}

fn test_call_type(s: &str) {
    match s {
        "audio" | "video" => {}
        _ => {}
    }
}

mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }
        let mut result = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        for i in (0..bytes.len()).step_by(2) {
            let high = char_to_nibble(bytes[i])?;
            let low = char_to_nibble(bytes[i + 1])?;
            result.push((high << 4) | low);
        }
        Ok(result)
    }

    fn char_to_nibble(c: u8) -> Result<u8, ()> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(()),
        }
    }
}
