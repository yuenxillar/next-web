//! Message codec for encoding and decoding SockJS frames.

use std::error::Error;

use crate::sockjs::SockJsFrame;

pub mod jackson2_message_codec;

pub use jackson2_message_codec::Jackson2SockJsMessageCodec;

/// Trait for encoding and decoding SockJS protocol frames.
///
/// Implementations of this trait handle the serialization and deserialization
/// of SockJS frames to/from their wire format.
///
/// # Examples
///
/// ```rust
/// use next_web_websocket::sockjs::codec::SockJsMessageCodec;
/// use next_web_websocket::sockjs::codec::Jackson2SockJsMessageCodec;
/// use next_web_websocket::sockjs::SockJsFrame;
///
/// let codec = Jackson2SockJsMessageCodec;
/// let frame = SockJsFrame::Open;
/// let encoded = codec.encode(&frame).unwrap();
/// assert_eq!(encoded, "o");
/// ```
#[next_web_core::async_trait]
pub trait SockJsMessageCodec: Send + Sync {
    /// Encodes a SockJS frame into its wire format string.
    ///
    /// # Arguments
    ///
    /// * `frame` - The frame to encode
    ///
    /// # Returns
    ///
    /// The encoded string representation, or an error if encoding fails.
    fn encode(&self, frame: &SockJsFrame) -> Result<String, Box<dyn Error>>;

    /// Decodes a wire format string into a SockJS frame.
    ///
    /// # Arguments
    ///
    /// * `data` - The encoded frame data
    ///
    /// # Returns
    ///
    /// The decoded frame, or an error if decoding fails.
    fn decode(&self, data: &str) -> Result<SockJsFrame, Box<dyn Error>>;
}
