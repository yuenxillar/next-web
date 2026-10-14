//! JSON-based message codec for SockJS frames.
//!
//! This codec uses `serde_json` for serialization, matching the behavior
//! of Spring Framework's `Jackson2SockJsMessageCodec`.

use std::error::Error;

use serde_json;

use crate::sockjs::{SockJsFrame, codec::SockJsMessageCodec};

/// JSON-based codec for SockJS frames using serde_json.
///
/// This codec implements the standard SockJS protocol encoding:
/// - Open: `o`
/// - Heartbeat: `h`
/// - Message: `a["msg1","msg2",...]`
/// - Close: `c[code,"reason"]`
#[derive(Debug, Clone, Copy, Default)]
pub struct Jackson2SockJsMessageCodec;

impl SockJsMessageCodec for Jackson2SockJsMessageCodec {
    fn encode(&self, frame: &SockJsFrame) -> Result<String, Box<dyn Error>> {
        match frame {
            SockJsFrame::Open => Ok("o".to_string()),
            SockJsFrame::Heartbeat => Ok("h".to_string()),
            SockJsFrame::Message(messages) => {
                let json = serde_json::to_string(messages)?;
                Ok(format!("a{}", json))
            }
            SockJsFrame::Close { code, reason } => {
                // Escape reason string and format as JSON array
                let reason_json = serde_json::to_string(reason)?;
                Ok(format!("c[{},{}]", code, reason_json))
            }
        }
    }

    fn decode(&self, data: &str) -> Result<SockJsFrame, Box<dyn Error>> {
        if data.is_empty() {
            return Err("Empty frame data".into());
        }

        let frame_type = &data[0..1];
        match frame_type {
            "o" => Ok(SockJsFrame::Open),
            "h" => Ok(SockJsFrame::Heartbeat),
            "a" => {
                // Message frame: a["msg1","msg2",...]
                if data.len() < 2 {
                    return Err("Invalid message frame: too short".into());
                }
                let json_data = &data[1..];
                let messages: Vec<String> = serde_json::from_str(json_data)?;
                Ok(SockJsFrame::Message(messages))
            }
            "c" => {
                // Close frame: c[code,"reason"]
                if data.len() < 2 {
                    return Err("Invalid close frame: too short".into());
                }
                let json_data = &data[1..];
                let parts: serde_json::Value = serde_json::from_str(json_data)?;

                if let Some(array) = parts.as_array() {
                    if array.len() >= 2 {
                        let code = array[0]
                            .as_u64()
                            .ok_or_else(|| "Close code must be a number")?
                            as u16;
                        let reason = array[1]
                            .as_str()
                            .ok_or_else(|| "Close reason must be a string")?
                            .to_string();
                        return Ok(SockJsFrame::Close { code, reason });
                    }
                }
                Err("Invalid close frame format".into())
            }
            _ => Err(format!("Unknown frame type: {}", frame_type).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_open() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = SockJsFrame::Open;
        assert_eq!(codec.encode(&frame).unwrap(), "o");
    }

    #[test]
    fn test_encode_heartbeat() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = SockJsFrame::Heartbeat;
        assert_eq!(codec.encode(&frame).unwrap(), "h");
    }

    #[test]
    fn test_encode_message_single() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = SockJsFrame::Message(vec!["hello".to_string()]);
        assert_eq!(codec.encode(&frame).unwrap(), "a[\"hello\"]");
    }

    #[test]
    fn test_encode_message_multiple() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = SockJsFrame::Message(vec!["hello".to_string(), "world".to_string()]);
        assert_eq!(codec.encode(&frame).unwrap(), "a[\"hello\",\"world\"]");
    }

    #[test]
    fn test_encode_close() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = SockJsFrame::Close {
            code: 3000,
            reason: "Go away!".to_string(),
        };
        assert_eq!(codec.encode(&frame).unwrap(), "c[3000,\"Go away!\"]");
    }

    #[test]
    fn test_decode_open() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = codec.decode("o").unwrap();
        assert_eq!(frame, SockJsFrame::Open);
    }

    #[test]
    fn test_decode_heartbeat() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = codec.decode("h").unwrap();
        assert_eq!(frame, SockJsFrame::Heartbeat);
    }

    #[test]
    fn test_decode_message() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = codec.decode("a[\"hello\",\"world\"]").unwrap();
        assert_eq!(
            frame,
            SockJsFrame::Message(vec!["hello".to_string(), "world".to_string()])
        );
    }

    #[test]
    fn test_decode_close() {
        let codec = Jackson2SockJsMessageCodec;
        let frame = codec.decode("c[3000,\"Go away!\"]").unwrap();
        assert_eq!(
            frame,
            SockJsFrame::Close {
                code: 3000,
                reason: "Go away!".to_string()
            }
        );
    }

    #[test]
    fn test_roundtrip() {
        let codec = Jackson2SockJsMessageCodec;
        let frames = vec![
            SockJsFrame::Open,
            SockJsFrame::Heartbeat,
            SockJsFrame::Message(vec!["test".to_string()]),
            SockJsFrame::Close {
                code: 3000,
                reason: "test".to_string(),
            },
        ];

        for frame in frames {
            let encoded = codec.encode(&frame).unwrap();
            let decoded = codec.decode(&encoded).unwrap();
            assert_eq!(frame, decoded);
        }
    }
}
