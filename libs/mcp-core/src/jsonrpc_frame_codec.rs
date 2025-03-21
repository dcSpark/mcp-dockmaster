use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

/// JsonRpcFrameCodec processes JSON-RPC frames from a byte stream.
/// It handles newline-delimited JSON messages.
pub struct JsonRpcFrameCodec;

impl Decoder for JsonRpcFrameCodec {
    type Item = BytesMut;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Look for a newline in the buffer
        if let Some(pos) = src.iter().position(|&b| b == b'\n') {
            // Split the buffer at the newline
            let frame = src.split_to(pos + 1);
            
            // Return all but the newline
            let mut result = BytesMut::new();
            result.extend_from_slice(&frame[..frame.len() - 1]);
            
            Ok(Some(result))
        } else {
            // No complete frame yet
            Ok(None)
        }
    }
}

impl Encoder<Vec<u8>> for JsonRpcFrameCodec {
    type Error = std::io::Error;
    
    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Add the data to the buffer
        dst.reserve(item.len() + 1);
        dst.put_slice(&item);
        dst.put_u8(b'\n');
        Ok(())
    }
} 