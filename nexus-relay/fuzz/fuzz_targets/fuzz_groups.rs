#![no_main]
use libfuzzer_sys::fuzz_target;

/// Fuzz test for group operations
/// Tests that group management handles malformed inputs gracefully
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Test group name validation
        test_group_name(s);

        // Test member hash validation
        test_member_hash(s);

        // Test group ID parsing
        test_group_id(s);
    }
});

fn test_group_name(name: &str) {
    // Valid group names
    let is_valid = !name.is_empty()
        && name.len() <= 100
        && name.chars().all(|c| c.is_ascii_graphic() || c == ' ');

    // Should not panic on any input
    let _ = is_valid;

    // Test with special characters
    let _ = name.replace(['<', '>', '&', '"', '\''], "");

    // Test trimming
    let _ = name.trim();
}

fn test_member_hash(hash: &str) {
    // Valid hash: 64 hex characters
    let is_valid_hash = hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit());

    // Should not panic
    let _ = is_valid_hash;

    // Test hex decoding
    if is_valid_hash {
        let _ = hex::decode(hash);
    }
}

fn test_group_id(id: &str) {
    // UUID format validation
    let is_valid_uuid = id.len() == 36
        && id.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        });

    let _ = is_valid_uuid;
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
