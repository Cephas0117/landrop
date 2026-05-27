use bytes::{BufMut, BytesMut};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

use crate::WireMessage;

pub const MAX_FRAME_SIZE: usize = 2 * 1024 * 1024; // 2MB
const HEADER_LEN: usize = 4; // u32 big-endian length prefix

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("frame too large: {0} bytes (max {MAX_FRAME_SIZE})")]
    FrameTooLarge(usize),
    #[error("msgpack encode error: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("msgpack decode error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FrameCodec;

impl Decoder for FrameCodec {
    type Item = WireMessage;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < HEADER_LEN {
            return Ok(None);
        }
        let frame_len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
        if frame_len > MAX_FRAME_SIZE {
            return Err(CodecError::FrameTooLarge(frame_len));
        }
        if src.len() < HEADER_LEN + frame_len {
            src.reserve(HEADER_LEN + frame_len - src.len());
            return Ok(None);
        }
        let _ = src.split_to(HEADER_LEN);
        let data = src.split_to(frame_len);
        let msg: WireMessage = rmp_serde::from_slice(&data)?;
        Ok(Some(msg))
    }
}

impl Encoder<WireMessage> for FrameCodec {
    type Error = CodecError;

    fn encode(&mut self, item: WireMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = rmp_serde::to_vec(&item)?;
        if payload.len() > MAX_FRAME_SIZE {
            return Err(CodecError::FrameTooLarge(payload.len()));
        }
        dst.reserve(HEADER_LEN + payload.len());
        dst.put_u32(payload.len() as u32);
        dst.put_slice(&payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Capabilities, Hello, WireMessage};
    use uuid::Uuid;

    fn make_hello() -> WireMessage {
        WireMessage::Hello(Hello {
            device_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            device_name: "test".into(),
            os: "linux".into(),
            cert_fingerprint: "fp".into(),
            capabilities: Capabilities::default(),
        })
    }

    #[test]
    fn encode_decode_roundtrip() {
        let mut codec = FrameCodec;
        let mut buf = BytesMut::new();
        let msg = make_hello();
        codec.encode(msg, &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();
        assert!(matches!(decoded, WireMessage::Hello(_)));
    }

    #[test]
    fn partial_frame_returns_none() {
        let mut codec = FrameCodec;
        let mut buf = BytesMut::new();
        codec.encode(make_hello(), &mut buf).unwrap();
        let full = buf.clone();
        let mut partial = BytesMut::from(&full[..full.len() - 2]);
        let result = codec.decode(&mut partial).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn oversized_frame_errors() {
        let mut codec = FrameCodec;
        let mut buf = BytesMut::new();
        buf.put_u32((MAX_FRAME_SIZE + 1) as u32);
        assert!(codec.decode(&mut buf).is_err());
    }
}
