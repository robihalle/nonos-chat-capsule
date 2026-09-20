extern crate alloc;
#[path = "../capsule/src/protocol.rs"]
pub mod protocol;
#[cfg(test)]
mod tests {
    use super::protocol::*;
    #[test]
    fn reject_endpoint_injection() {
        for s in [
            "http://api.example/v1",
            "https://x@evil/v1",
            "https://api.example/\r\nX: y",
            "https://api.example/v1?x=1",
            "https://api.example:0/v1",
            "https://[::1]/v1",
        ] {
            assert!(Endpoint::parse(s).is_err(), "{s}");
        }
    }
    #[test]
    fn normalize_and_keep_custom_prefix() {
        let e = Endpoint::parse(" https://api.example:8443/custom/v1/ ").unwrap();
        assert_eq!(e.base, "/custom/v1");
        assert_eq!(e.port, 8443);
        let r = request(
            &e,
            "secret",
            "model",
            &[Message {
                role: "user",
                text: "Hello \"world\"\ncafé".into(),
            }],
            false,
        )
        .unwrap();
        let text = String::from_utf8(r).unwrap();
        let (h, b) = text.split_once("\r\n\r\n").unwrap();
        assert!(h.starts_with("POST /custom/v1/chat/completions HTTP/1.1"));
        let j: serde_json::Value = serde_json::from_str(b).unwrap();
        assert_eq!(j["messages"][0]["content"], "Hello \"world\"\ncafé");
        assert!(h.contains(&format!("Content-Length: {}", b.len())));
    }
    #[test]
    fn reject_key_injection() {
        let e = Endpoint::parse("https://a").unwrap();
        assert!(request(&e, "key\r\nX: x", "m", &[], true).is_err());
    }
    #[test]
    fn partial_content_length() {
        let r = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        for i in 0..r.len() {
            assert!(response(&r[..i]).unwrap().is_none());
        }
        assert_eq!(response(r).unwrap().unwrap().body, b"{}");
    }
    #[test]
    fn partial_chunked() {
        let r = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n";
        for i in 0..r.len() {
            assert!(response(&r[..i]).unwrap().is_none());
        }
        assert_eq!(response(r).unwrap().unwrap().body, b"{}");
    }
    #[test]
    fn ambiguous_framing() {
        assert!(response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\n{}"
        )
        .is_err());
    }
    #[test]
    fn bound_responses() {
        assert!(response(b"HTTP/1.1 200 OK\r\nContent-Length: 9999999\r\n\r\n").is_err());
    }
    #[test]
    fn parse_assistant_unicode() {
        let r = Response {
            status: 200,
            body: br#"{"choices":[{"message":{"content":"caf\u00e9"}}]}"#.to_vec(),
        };
        assert_eq!(answer(r, false).unwrap(), "café");
    }
    #[test]
    fn error_does_not_echo_key() {
        let r = Response {
            status: 401,
            body: b"secret-api-key".to_vec(),
        };
        assert!(!answer(r, false).unwrap_err().contains("secret"));
    }
    #[test]
    fn model_list() {
        let r = Response {
            status: 200,
            body: br#"{"data":[{"id":"test-model"}]}"#.to_vec(),
        };
        assert!(answer(r, true).unwrap().contains("test-model"));
    }
}

#[path = "../compat/virtqueue_layout.rs"]
pub mod virtqueue_layout;
