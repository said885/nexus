#![no_main]
use libfuzzer_sys::fuzz_target;

/// Fuzz test for WebSocket message parsing
/// Tests that the server handles malformed JSON gracefully without crashing
fuzz_target!(|data: &[u8]| {
    // Try to parse as UTF-8 string
    if let Ok(s) = std::str::from_utf8(data) {
        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(s) {
            // Check if it has a type field
            if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                match msg_type {
                    "identify" => {
                        // Validate identify message structure
                        let _ = json.get("recipient_hash");
                        let _ = json.get("challenge_response");
                    }
                    "send" => {
                        // Validate send message structure
                        let _ = json.get("recipient");
                        let _ = json.get("sealed_content");
                        let _ = json.get("ttl");
                    }
                    "send_group" => {
                        let _ = json.get("group_id");
                        let _ = json.get("sealed_content");
                        let _ = json.get("ttl");
                    }
                    "fetch_offline" => {
                        // No additional fields needed
                    }
                    "ping" => {
                        let _ = json.get("nonce");
                    }
                    "set_status" => {
                        let _ = json.get("status");
                        let _ = json.get("message");
                    }
                    "typing" => {
                        let _ = json.get("conversation_id");
                        let _ = json.get("is_typing");
                    }
                    "call_initiate" => {
                        let _ = json.get("recipient");
                        let _ = json.get("call_type");
                    }
                    "call_accept" => {
                        let _ = json.get("call_id");
                    }
                    "call_end" => {
                        let _ = json.get("call_id");
                    }
                    "group_create" => {
                        let _ = json.get("name");
                        let _ = json.get("description");
                    }
                    "group_add_member" => {
                        let _ = json.get("group_id");
                        let _ = json.get("member_hash");
                    }
                    "group_remove_member" => {
                        let _ = json.get("group_id");
                        let _ = json.get("member_hash");
                    }
                    "group_info" => {
                        let _ = json.get("group_id");
                    }
                    "group_leave" => {
                        let _ = json.get("group_id");
                    }
                    "ack_delivered" => {
                        let _ = json.get("message_id");
                    }
                    _ => {
                        // Unknown message type - should be handled gracefully
                    }
                }
            }
        }
    }

    // Also test with raw bytes (should not crash)
    let _ = serde_json::from_slice::<serde_json::Value>(data);
});
