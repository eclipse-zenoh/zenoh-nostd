//! Serialization/deserialization API analogous to zenoh-pico's `ze_serializer_*` / `ze_deserializer_*`.
//!
//! Wire format (little-endian):
//! - Integers: fixed-width little-endian
//! - Floats: IEEE 754 binary representation (little-endian)
//! - Booleans: single byte (0x00 or 0x01)
//! - Strings: VLE-encoded u32 length + UTF-8 bytes
//! - Slices: VLE-encoded u32 length + raw bytes
//! - Sequence length: VLE-encoded u32

#![allow(dead_code)]

use core::fmt;

/// Default capacity for serialization buffers.
pub const DEFAULT_SERIALIZATION_CAPACITY: usize = 1024;

/// Errors that can occur during serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializeError {
    /// The output buffer does not have enough remaining capacity.
    BufferFull,
    /// The value is too large to fit in the wire format (e.g. a VLE u32 overflow).
    ValueTooLarge,
}

impl fmt::Display for SerializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferFull => write!(f, "serialization buffer is full"),
            Self::ValueTooLarge => write!(f, "value too large to serialize"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SerializeError {}

/// Errors that can occur during deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeserializeError {
    /// Not enough data in the buffer.
    InsufficientData,
    /// Encountered invalid UTF-8 data.
    InvalidUtf8,
    /// Encountered an invalid or unexpected value.
    InvalidValue,
}

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData => write!(f, "insufficient data for deserialization"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 data"),
            Self::InvalidValue => write!(f, "invalid value encountered"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DeserializeError {}

// ---------------------------------------------------------------------------
// ZSerializer
// ---------------------------------------------------------------------------

/// Serializes typed data into a byte buffer.
///
/// Wire format (little-endian):
/// - Integers: fixed-width little-endian bytes
/// - Floats: IEEE 754 binary representation (little-endian)
/// - Booleans: single byte (0x00 or 0x01)
/// - Strings: VLE-encoded u32 byte length + UTF-8 bytes
/// - Slices: VLE-encoded u32 byte length + raw bytes
/// - Sequence length: VLE-encoded u32
pub struct ZSerializer<'a> {
    buffer: &'a mut [u8],
    pos: usize,
}

impl<'a> ZSerializer<'a> {
    /// Creates a new serializer that writes into `buffer`.
    ///
    /// Returns `None` if `buffer` is empty (zero remaining capacity).
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer, pos: 0 }
    }

    /// Number of bytes written so far.
    pub fn len(&self) -> usize {
        self.pos
    }

    /// Returns `true` if no bytes have been written yet.
    pub fn is_empty(&self) -> bool {
        self.pos == 0
    }

    /// Remaining writable capacity.
    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    fn ensure(&self, needed: usize) -> Result<(), SerializeError> {
        if self.remaining() < needed {
            Err(SerializeError::BufferFull)
        } else {
            Ok(())
        }
    }

    fn write_bytes(&mut self, data: &[u8]) -> Result<(), SerializeError> {
        self.ensure(data.len())?;
        self.buffer[self.pos..self.pos + data.len()].copy_from_slice(data);
        self.pos += data.len();
        Ok(())
    }

    /// Writes a VLE (Variable Length Encoding) u32.
    ///
    /// The encoding uses 7-bit chunks with a continuation bit (MSB).
    /// This is the standard Zenoh VLE format.
    fn write_vle_u32(&mut self, mut v: u32) -> Result<(), SerializeError> {
        loop {
            if v < 0x80 {
                self.ensure(1)?;
                self.buffer[self.pos] = v as u8;
                self.pos += 1;
                return Ok(());
            }
            let byte = (v as u8) | 0x80;
            self.ensure(1)?;
            self.buffer[self.pos] = byte;
            self.pos += 1;
            v >>= 7;
        }
    }

    /// Serializes a `u8` value (1 byte).
    pub fn serialize_uint8(&mut self, v: u8) -> Result<(), SerializeError> {
        self.write_bytes(&[v])
    }

    /// Serializes a `u16` value (2 bytes, little-endian).
    pub fn serialize_uint16(&mut self, v: u16) -> Result<(), SerializeError> {
        self.write_bytes(&v.to_le_bytes())
    }

    /// Serializes a `u32` value (4 bytes, little-endian).
    pub fn serialize_uint32(&mut self, v: u32) -> Result<(), SerializeError> {
        self.write_bytes(&v.to_le_bytes())
    }

    /// Serializes a `u64` value (8 bytes, little-endian).
    pub fn serialize_uint64(&mut self, v: u64) -> Result<(), SerializeError> {
        self.write_bytes(&v.to_le_bytes())
    }

    /// Serializes an `f32` value (4 bytes, IEEE 754 binary32, little-endian).
    pub fn serialize_float32(&mut self, v: f32) -> Result<(), SerializeError> {
        self.write_bytes(&v.to_bits().to_le_bytes())
    }

    /// Serializes an `f64` value (8 bytes, IEEE 754 binary64, little-endian).
    pub fn serialize_float64(&mut self, v: f64) -> Result<(), SerializeError> {
        self.write_bytes(&v.to_bits().to_le_bytes())
    }

    /// Serializes a `bool` value (1 byte: 0x00 or 0x01).
    pub fn serialize_bool(&mut self, v: bool) -> Result<(), SerializeError> {
        self.write_bytes(&[v as u8])
    }

    /// Serializes a string as VLE-encoded length + UTF-8 bytes.
    pub fn serialize_str(&mut self, s: &str) -> Result<(), SerializeError> {
        let len = s.len();
        let len32 = u32::try_from(len).map_err(|_| SerializeError::ValueTooLarge)?;
        self.write_vle_u32(len32)?;
        self.write_bytes(s.as_bytes())
    }

    /// Serializes a byte slice as VLE-encoded length + raw bytes.
    pub fn serialize_slice(&mut self, data: &[u8]) -> Result<(), SerializeError> {
        let len = data.len();
        let len32 = u32::try_from(len).map_err(|_| SerializeError::ValueTooLarge)?;
        self.write_vle_u32(len32)?;
        self.write_bytes(data)
    }

    /// Serializes a sequence length as VLE-encoded u32.
    pub fn serialize_sequence_length(&mut self, len: u32) -> Result<(), SerializeError> {
        self.write_vle_u32(len)
    }

    /// Returns a slice of the written bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.pos]
    }

    /// Consumes the serializer and returns the written bytes as a slice.
    pub fn finish(self) -> &'a [u8] {
        &self.buffer[..self.pos]
    }
}

// ---------------------------------------------------------------------------
// ZDeserializer
// ---------------------------------------------------------------------------

/// Deserializes typed data from a byte buffer.
pub struct ZDeserializer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ZDeserializer<'a> {
    /// Creates a new deserializer from a byte slice.
    pub fn from_bytes(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Returns the number of unread bytes remaining.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Returns `true` if no bytes remain to be read.
    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Returns the current read position.
    pub fn position(&self) -> usize {
        self.pos
    }

    fn ensure(&self, needed: usize) -> Result<(), DeserializeError> {
        if self.remaining() < needed {
            Err(DeserializeError::InsufficientData)
        } else {
            Ok(())
        }
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], DeserializeError> {
        self.ensure(len)?;
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn read_vle_u32(&mut self) -> Result<u32, DeserializeError> {
        let mut v: u32 = 0;
        let mut shift: u32 = 0;
        loop {
            let byte = self
                .data
                .get(self.pos)
                .copied()
                .ok_or(DeserializeError::InsufficientData)?;
            self.pos += 1;
            v |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(v);
            }
            shift += 7;
            // u32 VLE is at most 5 bytes (5*7 = 35 bits)
            if shift >= 35 {
                return Err(DeserializeError::InvalidValue);
            }
        }
    }

    /// Deserializes a `u8` value (1 byte).
    pub fn deserialize_uint8(&mut self) -> Result<u8, DeserializeError> {
        let bytes = self.read_bytes(1)?;
        Ok(bytes[0])
    }

    /// Deserializes a `u16` value (2 bytes, little-endian).
    pub fn deserialize_uint16(&mut self) -> Result<u16, DeserializeError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Deserializes a `u32` value (4 bytes, little-endian).
    pub fn deserialize_uint32(&mut self) -> Result<u32, DeserializeError> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Deserializes a `u64` value (8 bytes, little-endian).
    pub fn deserialize_uint64(&mut self) -> Result<u64, DeserializeError> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Deserializes an `f32` value (4 bytes, IEEE 754 binary32, little-endian).
    pub fn deserialize_float32(&mut self) -> Result<f32, DeserializeError> {
        let bits = self.deserialize_uint32()?;
        Ok(f32::from_bits(bits))
    }

    /// Deserializes an `f64` value (8 bytes, IEEE 754 binary64, little-endian).
    pub fn deserialize_float64(&mut self) -> Result<f64, DeserializeError> {
        let bits = self.deserialize_uint64()?;
        Ok(f64::from_bits(bits))
    }

    /// Deserializes a `bool` value (1 byte: 0x00 or 0x01).
    pub fn deserialize_bool(&mut self) -> Result<bool, DeserializeError> {
        let byte = self.deserialize_uint8()?;
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DeserializeError::InvalidValue),
        }
    }

    /// Deserializes a string (VLE-encoded length + UTF-8 bytes).
    pub fn deserialize_str(&mut self) -> Result<&'a str, DeserializeError> {
        let len = self.read_vle_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        core::str::from_utf8(bytes).map_err(|_| DeserializeError::InvalidUtf8)
    }

    /// Deserializes a byte slice (VLE-encoded length + raw bytes).
    pub fn deserialize_slice(&mut self) -> Result<&'a [u8], DeserializeError> {
        let len = self.read_vle_u32()? as usize;
        self.read_bytes(len)
    }

    /// Deserializes a sequence length (VLE-encoded u32).
    pub fn deserialize_sequence_length(&mut self) -> Result<u32, DeserializeError> {
        self.read_vle_u32()
    }
}

// ---------------------------------------------------------------------------
// Convenience functions (stack-allocated owned buffers)
// ---------------------------------------------------------------------------

/// A fixed-capacity serialization buffer.
pub type SerializationBuf = heapless::Vec<u8, DEFAULT_SERIALIZATION_CAPACITY>;

fn into_buf(written: &[u8]) -> Result<SerializationBuf, SerializeError> {
    let mut buf = SerializationBuf::new();
    buf.extend_from_slice(written)
        .map_err(|_| SerializeError::BufferFull)?;
    Ok(buf)
}

/// Serializes a single `u8` into an owned buffer.
pub fn serialize_uint8(v: u8) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 1];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_uint8(v)?;
    into_buf(ser.as_bytes())
}

/// Serializes a single `u16` into an owned buffer.
pub fn serialize_uint16(v: u16) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 2];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_uint16(v)?;
    into_buf(ser.as_bytes())
}

/// Serializes a single `u32` into an owned buffer.
pub fn serialize_uint32(v: u32) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 4];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_uint32(v)?;
    into_buf(ser.as_bytes())
}

/// Serializes a single `u64` into an owned buffer.
pub fn serialize_uint64(v: u64) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 8];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_uint64(v)?;
    into_buf(ser.as_bytes())
}

/// Serializes a single `f32` into an owned buffer.
pub fn serialize_float32(v: f32) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 4];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_float32(v)?;
    into_buf(ser.as_bytes())
}

/// Serializes a single `f64` into an owned buffer.
pub fn serialize_float64(v: f64) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 8];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_float64(v)?;
    into_buf(ser.as_bytes())
}

/// Serializes a single `bool` into an owned buffer.
pub fn serialize_bool(v: bool) -> Result<SerializationBuf, SerializeError> {
    let mut tmp = [0u8; 1];
    let mut ser = ZSerializer::new(&mut tmp);
    ser.serialize_bool(v)?;
    into_buf(ser.as_bytes())
}

/// Deserializes a single `u8` from a byte slice.
pub fn deserialize_uint8(data: &[u8]) -> Result<u8, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_uint8()
}

/// Deserializes a single `u16` from a byte slice.
pub fn deserialize_uint16(data: &[u8]) -> Result<u16, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_uint16()
}

/// Deserializes a single `u32` from a byte slice.
pub fn deserialize_uint32(data: &[u8]) -> Result<u32, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_uint32()
}

/// Deserializes a single `u64` from a byte slice.
pub fn deserialize_uint64(data: &[u8]) -> Result<u64, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_uint64()
}

/// Deserializes a single `f32` from a byte slice.
pub fn deserialize_float32(data: &[u8]) -> Result<f32, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_float32()
}

/// Deserializes a single `f64` from a byte slice.
pub fn deserialize_float64(data: &[u8]) -> Result<f64, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_float64()
}

/// Deserializes a single `bool` from a byte slice.
pub fn deserialize_bool(data: &[u8]) -> Result<bool, DeserializeError> {
    let mut de = ZDeserializer::from_bytes(data);
    de.deserialize_bool()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Roundtrip tests ----

    #[test]
    fn test_u8_roundtrip() {
        let val: u8 = 0xAB;
        let mut buf = [0u8; 1];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_uint8(val).unwrap();
        assert_eq!(ser.len(), 1);
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_uint8().unwrap(), val);
    }

    #[test]
    fn test_u16_roundtrip() {
        let val: u16 = 0xABCD;
        let mut buf = [0u8; 2];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_uint16(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_uint16().unwrap(), val);
    }

    #[test]
    fn test_u32_roundtrip() {
        let val: u32 = 0xDEAD_BEEF;
        let mut buf = [0u8; 4];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_uint32(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_uint32().unwrap(), val);
    }

    #[test]
    fn test_u64_roundtrip() {
        let val: u64 = 0xCAFE_BABE_DEAD_BEEF;
        let mut buf = [0u8; 8];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_uint64(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_uint64().unwrap(), val);
    }

    #[test]
    fn test_f32_roundtrip() {
        let val: f32 = 3.141_592_7;
        let mut buf = [0u8; 4];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_float32(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_float32().unwrap(), val);
    }

    #[test]
    fn test_f64_roundtrip() {
        let val: f64 = 2.718_281_828_459_045;
        let mut buf = [0u8; 8];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_float64(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_float64().unwrap(), val);
    }

    #[test]
    fn test_bool_roundtrip() {
        for &val in &[false, true] {
            let mut buf = [0u8; 1];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_bool(val).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            assert_eq!(de.deserialize_bool().unwrap(), val);
        }
    }

    #[test]
    fn test_str_roundtrip() {
        let val = "hello zenoh!";
        let mut buf = [0u8; 64];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_str(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_str().unwrap(), val);
    }

    #[test]
    fn test_str_empty() {
        let val = "";
        let mut buf = [0u8; 8];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_str(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_str().unwrap(), val);
    }

    #[test]
    fn test_slice_roundtrip() {
        let val: &[u8] = &[1, 2, 3, 4, 5];
        let mut buf = [0u8; 16];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_slice(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_slice().unwrap(), val);
    }

    #[test]
    fn test_slice_empty() {
        let val: &[u8] = &[];
        let mut buf = [0u8; 8];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_slice(val).unwrap();
        let mut de = ZDeserializer::from_bytes(ser.finish());
        assert_eq!(de.deserialize_slice().unwrap(), val);
    }

    #[test]
    fn test_sequence_length_roundtrip() {
        let vals: &[u32] = &[0, 1, 127, 128, 16383, 16384, 2097151, 268_435_455];
        for &len in vals {
            let mut buf = [0u8; 8];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_sequence_length(len).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            assert_eq!(
                de.deserialize_sequence_length().unwrap(),
                len,
                "failed for len={}",
                len
            );
        }
    }

    // ---- Roundtrip: convenience functions ----

    #[test]
    fn test_convenience_u8_roundtrip() {
        let val: u8 = 42;
        let buf = serialize_uint8(val).unwrap();
        assert_eq!(deserialize_uint8(&buf).unwrap(), val);
    }

    #[test]
    fn test_convenience_u16_roundtrip() {
        let val: u16 = 0x1234;
        let buf = serialize_uint16(val).unwrap();
        assert_eq!(deserialize_uint16(&buf).unwrap(), val);
    }

    #[test]
    fn test_convenience_u32_roundtrip() {
        let val: u32 = 0x1234_5678;
        let buf = serialize_uint32(val).unwrap();
        assert_eq!(deserialize_uint32(&buf).unwrap(), val);
    }

    #[test]
    fn test_convenience_u64_roundtrip() {
        let val: u64 = 0x1234_5678_9ABC_DEF0;
        let buf = serialize_uint64(val).unwrap();
        assert_eq!(deserialize_uint64(&buf).unwrap(), val);
    }

    #[test]
    fn test_convenience_f32_roundtrip() {
        let val: f32 = -3.14;
        let buf = serialize_float32(val).unwrap();
        assert_eq!(deserialize_float32(&buf).unwrap(), val);
    }

    #[test]
    fn test_convenience_f64_roundtrip() {
        let val: f64 = 1.0e-100;
        let buf = serialize_float64(val).unwrap();
        assert_eq!(deserialize_float64(&buf).unwrap(), val);
    }

    #[test]
    fn test_convenience_bool_roundtrip() {
        for &val in &[false, true] {
            let buf = serialize_bool(val).unwrap();
            assert_eq!(deserialize_bool(&buf).unwrap(), val);
        }
    }

    // ---- Multi-value sequence ----

    #[test]
    fn test_multi_value_sequence() {
        let mut buf = [0u8; 64];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_uint8(1).unwrap();
        ser.serialize_uint16(0x0203).unwrap();
        ser.serialize_uint32(0x0405_0607).unwrap();
        ser.serialize_bool(true).unwrap();
        ser.serialize_str("test").unwrap();

        let data = ser.finish();
        let mut de = ZDeserializer::from_bytes(data);
        assert_eq!(de.deserialize_uint8().unwrap(), 1);
        assert_eq!(de.deserialize_uint16().unwrap(), 0x0203);
        assert_eq!(de.deserialize_uint32().unwrap(), 0x0405_0607);
        assert_eq!(de.deserialize_bool().unwrap(), true);
        assert_eq!(de.deserialize_str().unwrap(), "test");
        assert!(de.is_empty());
    }

    // ---- Error cases ----

    #[test]
    fn test_buffer_full() {
        let mut buf = [0u8; 1];
        let mut ser = ZSerializer::new(&mut buf);
        // Writing a u16 (2 bytes) into a 1-byte buffer should fail
        assert_eq!(
            ser.serialize_uint16(0xFFFF),
            Err(SerializeError::BufferFull)
        );
    }

    #[test]
    fn test_truncated_uint8() {
        let mut de = ZDeserializer::from_bytes(&[]);
        assert_eq!(
            de.deserialize_uint8(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_uint16() {
        let mut de = ZDeserializer::from_bytes(&[0x01]);
        assert_eq!(
            de.deserialize_uint16(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_uint32() {
        let mut de = ZDeserializer::from_bytes(&[0; 3]);
        assert_eq!(
            de.deserialize_uint32(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_uint64() {
        let mut de = ZDeserializer::from_bytes(&[0; 7]);
        assert_eq!(
            de.deserialize_uint64(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_float32() {
        let mut de = ZDeserializer::from_bytes(&[0; 3]);
        assert_eq!(
            de.deserialize_float32(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_float64() {
        let mut de = ZDeserializer::from_bytes(&[0; 7]);
        assert_eq!(
            de.deserialize_float64(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_str() {
        // VLE length says 5 but only 2 bytes follow
        let mut de = ZDeserializer::from_bytes(&[5, b'h', b'e']);
        assert_eq!(
            de.deserialize_str(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_truncated_slice() {
        // VLE length says 5 but only 2 bytes follow
        let mut de = ZDeserializer::from_bytes(&[5, 1, 2]);
        assert_eq!(
            de.deserialize_slice(),
            Err(DeserializeError::InsufficientData)
        );
    }

    #[test]
    fn test_invalid_bool() {
        let mut de = ZDeserializer::from_bytes(&[0x02]);
        assert_eq!(de.deserialize_bool(), Err(DeserializeError::InvalidValue));
    }

    #[test]
    fn test_invalid_utf8() {
        // VLE length 3, with invalid UTF-8 bytes.
        // Build manually: VLE(3) = 0x03 followed by bad bytes 0xFF, 0xFE, 0xFF
        let input = &[0x03, 0xFF, 0xFE, 0xFF];
        let mut de = ZDeserializer::from_bytes(input);
        assert_eq!(de.deserialize_str(), Err(DeserializeError::InvalidUtf8));
    }

    #[test]
    fn test_serializer_empty() {
        let mut buf = [0u8; 8];
        let ser = ZSerializer::new(&mut buf);
        assert!(ser.is_empty());
        assert_eq!(ser.len(), 0);
        assert_eq!(ser.remaining(), 8);
    }

    #[test]
    fn test_serializer_len_and_remaining() {
        let mut buf = [0u8; 8];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_uint8(1).unwrap();
        ser.serialize_uint16(2).unwrap();
        assert_eq!(ser.len(), 3);
        assert_eq!(ser.remaining(), 5);
    }

    #[test]
    fn test_deserializer_position() {
        let mut de = ZDeserializer::from_bytes(&[1, 2, 3, 4]);
        assert_eq!(de.position(), 0);
        de.deserialize_uint8().unwrap();
        assert_eq!(de.position(), 1);
        de.deserialize_uint16().unwrap();
        assert_eq!(de.position(), 3);
        de.deserialize_uint8().unwrap();
        assert_eq!(de.position(), 4);
        assert!(de.is_empty());
    }

    #[test]
    fn test_partial_read_then_full_read() {
        let mut buf = [0u8; 32];
        let mut ser = ZSerializer::new(&mut buf);
        ser.serialize_str("abc").unwrap();
        ser.serialize_str("defg").unwrap();

        let data = ser.finish();
        let mut de = ZDeserializer::from_bytes(data);
        assert_eq!(de.deserialize_str().unwrap(), "abc");
        assert_eq!(de.deserialize_str().unwrap(), "defg");
        assert!(de.is_empty());
    }

    // ---- Boundary values ----

    #[test]
    fn test_u8_boundaries() {
        for &v in &[u8::MIN, u8::MAX, 0, 1, 127, 128, 255] {
            let mut buf = [0u8; 1];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_uint8(v).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            assert_eq!(de.deserialize_uint8().unwrap(), v);
        }
    }

    #[test]
    fn test_u16_boundaries() {
        for &v in &[u16::MIN, u16::MAX, 0, 1, 255, 256, 0x00FF, 0xFF00] {
            let mut buf = [0u8; 2];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_uint16(v).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            assert_eq!(de.deserialize_uint16().unwrap(), v);
        }
    }

    #[test]
    fn test_u32_boundaries() {
        let vals = [
            u32::MIN,
            u32::MAX,
            0,
            1,
            0x0000_00FF,
            0x0000_FF00,
            0x00FF_0000,
            0xFF00_0000,
        ];
        for &v in &vals {
            let mut buf = [0u8; 4];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_uint32(v).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            assert_eq!(de.deserialize_uint32().unwrap(), v);
        }
    }

    #[test]
    fn test_u64_boundaries() {
        let vals = [
            u64::MIN,
            u64::MAX,
            0,
            1,
            0x0000_0000_0000_00FF,
            0xFF00_0000_0000_0000,
        ];
        for &v in &vals {
            let mut buf = [0u8; 8];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_uint64(v).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            assert_eq!(de.deserialize_uint64().unwrap(), v);
        }
    }

    #[test]
    fn test_f32_special_values() {
        let vals = [0.0_f32, -0.0_f32, f32::INFINITY, f32::NEG_INFINITY];
        for &v in &vals {
            let mut buf = [0u8; 4];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_float32(v).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            let result = de.deserialize_float32().unwrap();
            // Compare bits for NaN-avoidance
            assert_eq!(result.to_bits(), v.to_bits(), "failed for f32={:?}", v);
        }
    }

    #[test]
    fn test_f64_special_values() {
        let vals = [0.0_f64, -0.0_f64, f64::INFINITY, f64::NEG_INFINITY];
        for &v in &vals {
            let mut buf = [0u8; 8];
            let mut ser = ZSerializer::new(&mut buf);
            ser.serialize_float64(v).unwrap();
            let mut de = ZDeserializer::from_bytes(ser.finish());
            let result = de.deserialize_float64().unwrap();
            assert_eq!(result.to_bits(), v.to_bits(), "failed for f64={:?}", v);
        }
    }
}
