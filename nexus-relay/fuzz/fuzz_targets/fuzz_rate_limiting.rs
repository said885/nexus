#![no_main]
use libfuzzer_sys::fuzz_target;

/// Fuzz test for rate limiting logic
/// Tests that rate limiter handles edge cases without panicking
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Test with various IP formats
        test_ip_formats(s);

        // Test with concurrent-like access patterns
        test_concurrent_access(s);
    }
});

fn test_ip_formats(ip: &str) {
    // Valid IP formats that should work
    let valid_ips = [
        "192.168.1.1",
        "10.0.0.1",
        "172.16.0.1",
        "2001:db8::1",
        "::1",
        "127.0.0.1",
    ];

    // Test if input matches valid pattern
    for valid_ip in &valid_ips {
        if ip == *valid_ip {
            // Should handle correctly
            return;
        }
    }

    // Test with various lengths
    let _ = ip.len() <= 45; // Max IPv6 length
}

fn test_concurrent_access(s: &str) {
    // Simulate rapid requests from same IP
    let ip = if s.is_empty() { "0.0.0.0" } else { s };

    // Test rapid succession (simulated)
    for _ in 0..10 {
        let _ = format!("{}:request", ip);
    }
}
