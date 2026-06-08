/// A borrowed byte slice view (analogous to `z_slice_t` in zenoh-pico)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZSlice<'a> {
    data: &'a [u8],
}

impl<'a> ZSlice<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<'a> From<&'a [u8]> for ZSlice<'a> {
    fn from(data: &'a [u8]) -> Self {
        Self { data }
    }
}

impl<'a> From<&'a str> for ZSlice<'a> {
    fn from(s: &'a str) -> Self {
        Self { data: s.as_bytes() }
    }
}

/// Errors from `ZBytes` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZBytesError {
    /// The input data exceeds the maximum capacity of `ZBytes`.
    CapacityExceeded,
}

/// An owned byte buffer (analogous to `z_bytes_t` in zenoh-pico).
///
/// Uses a fixed-capacity `heapless::Vec` for `no_std` compatibility.
/// The default maximum capacity is `DEFAULT_CAPACITY` (4096 bytes).
/// Use the type alias `ZBytes<DEFAULT_CAPACITY>` for the default size,
/// or instantiate `ZBytes<N>` with a custom capacity.
pub type ZBytes = ZBytesGeneric<DEFAULT_CAPACITY>;

/// Default maximum capacity for `ZBytes`.
pub const DEFAULT_CAPACITY: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZBytesGeneric<const N: usize> {
    data: heapless::Vec<u8, N>,
}

impl<const N: usize> ZBytesGeneric<N> {
    pub fn new() -> Self {
        Self {
            data: heapless::Vec::new(),
        }
    }

    pub fn from_slice(slice: &[u8]) -> Result<Self, ZBytesError> {
        let mut data = heapless::Vec::new();
        data.extend_from_slice(slice)
            .map_err(|_| ZBytesError::CapacityExceeded)?;
        Ok(Self { data })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, ZBytesError> {
        Self::from_slice(s.as_bytes())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn to_str(&self) -> Result<&str, core::str::Utf8Error> {
        core::str::from_utf8(self.data.as_slice())
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn remaining_capacity(&self) -> usize {
        N - self.data.len()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn reader(&self) -> ZBytesReader<'_> {
        ZBytesReader::new(self.data.as_slice())
    }

    pub fn writer() -> ZBytesWriterGeneric<N> {
        ZBytesWriterGeneric::new()
    }
}

impl<const N: usize> Default for ZBytesGeneric<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> From<ZBytesWriterGeneric<N>> for ZBytesGeneric<N> {
    fn from(writer: ZBytesWriterGeneric<N>) -> Self {
        writer.finish()
    }
}

/// A reader for sequential byte access (analogous to `z_bytes_reader_t`).
#[derive(Debug, Clone)]
pub struct ZBytesReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ZBytesReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn read_byte(&mut self) -> Result<u8, ReadError> {
        let byte = *self.data.get(self.pos).ok_or(ReadError::InsufficientData)?;
        self.pos += 1;
        Ok(byte)
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), ReadError> {
        let end = self
            .pos
            .checked_add(buf.len())
            .ok_or(ReadError::InsufficientData)?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or(ReadError::InsufficientData)?;
        buf.copy_from_slice(slice);
        self.pos = end;
        Ok(())
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn remaining_slice(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn reset(&mut self) {
        self.pos = 0;
    }

    pub fn skip(&mut self, n: usize) -> Result<(), ReadError> {
        let new_pos = self.pos.checked_add(n).ok_or(ReadError::InsufficientData)?;
        if new_pos > self.data.len() {
            return Err(ReadError::InsufficientData);
        }
        self.pos = new_pos;
        Ok(())
    }

    pub fn into_inner(self) -> &'a [u8] {
        self.data
    }
}

/// Errors from `ZBytesReader` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// Not enough data available to fulfill the read request.
    InsufficientData,
}

/// A writer for building byte buffers (analogous to `z_bytes_writer_t`).
pub type ZBytesWriter = ZBytesWriterGeneric<DEFAULT_CAPACITY>;

#[derive(Debug, Clone)]
pub struct ZBytesWriterGeneric<const N: usize> {
    data: heapless::Vec<u8, N>,
}

impl<const N: usize> ZBytesWriterGeneric<N> {
    pub fn new() -> Self {
        Self {
            data: heapless::Vec::new(),
        }
    }

    pub fn write_byte(&mut self, b: u8) -> Result<(), WriteError> {
        self.data.push(b).map_err(|_| WriteError::CapacityExceeded)
    }

    pub fn write_slice(&mut self, data: &[u8]) -> Result<(), WriteError> {
        self.data
            .extend_from_slice(data)
            .map_err(|_| WriteError::CapacityExceeded)
    }

    pub fn finish(self) -> ZBytesGeneric<N> {
        ZBytesGeneric { data: self.data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn remaining_capacity(&self) -> usize {
        N - self.data.len()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl<const N: usize> Default for ZBytesWriterGeneric<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from `ZBytesWriter` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    /// The buffer has reached its maximum capacity.
    CapacityExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ZSlice tests ---

    #[test]
    fn test_zslice_empty() {
        let s = ZSlice::new(b"");
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.as_slice(), b"");
    }

    #[test]
    fn test_zslice_from_bytes() {
        let data = b"hello";
        let s = ZSlice::new(data);
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_slice(), b"hello");
    }

    #[test]
    fn test_zslice_from_str() {
        let s: ZSlice<'_> = "hello".into();
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_slice(), b"hello");
    }

    #[test]
    fn test_zslice_from_slice() {
        let s: ZSlice<'_> = (&b"world"[..]).into();
        assert_eq!(s.as_slice(), b"world");
    }

    // --- ZBytes tests ---

    #[test]
    fn test_zbytes_new_is_empty() {
        let b = ZBytes::new();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn test_zbytes_from_slice_roundtrip() {
        let data = b"hello world";
        let b = ZBytes::from_slice(data).unwrap();
        assert!(!b.is_empty());
        assert_eq!(b.len(), 11);
        assert_eq!(b.as_slice(), data);
    }

    #[test]
    fn test_zbytes_from_str_roundtrip() {
        let s = "hello zenoh";
        let b = ZBytes::from_str(s).unwrap();
        assert_eq!(b.len(), 11);
        assert_eq!(b.to_str(), Ok(s));
    }

    #[test]
    fn test_zbytes_empty_slice() {
        let b = ZBytes::from_slice(b"").unwrap();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert_eq!(b.as_slice(), b"");
    }

    #[test]
    fn test_zbytes_empty_str() {
        let b = ZBytes::from_str("").unwrap();
        assert!(b.is_empty());
        assert_eq!(b.to_str(), Ok(""));
    }

    #[test]
    fn test_zbytes_capacity_exceeded() {
        // Use a tiny capacity to test overflow
        let data = [0u8; 10];
        let result = ZBytesGeneric::<5>::from_slice(&data);
        assert_eq!(result, Err(ZBytesError::CapacityExceeded));
    }

    #[test]
    fn test_zbytes_default() {
        let b: ZBytes = ZBytes::default();
        assert!(b.is_empty());
    }

    #[test]
    fn test_zbytes_clear() {
        let mut b = ZBytes::from_slice(b"hello").unwrap();
        assert!(!b.is_empty());
        b.clear();
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
    }

    #[test]
    fn test_zbytes_capacity_info() {
        let b = ZBytesGeneric::<128>::new();
        assert_eq!(b.capacity(), 128);
        assert_eq!(b.remaining_capacity(), 128);
    }

    #[test]
    fn test_zbytes_generic_capacity() {
        let b = ZBytesGeneric::<64>::from_slice(b"test").unwrap();
        assert_eq!(b.capacity(), 64);
        assert_eq!(b.remaining_capacity(), 60);
    }

    // --- ZBytesReader tests ---

    #[test]
    fn test_reader_read_byte() {
        let b = ZBytes::from_slice(b"abc").unwrap();
        let mut r = b.reader();
        assert_eq!(r.read_byte(), Ok(b'a'));
        assert_eq!(r.read_byte(), Ok(b'b'));
        assert_eq!(r.read_byte(), Ok(b'c'));
        assert_eq!(r.read_byte(), Err(ReadError::InsufficientData));
    }

    #[test]
    fn test_reader_read_exact() {
        let b = ZBytes::from_slice(b"abcdef").unwrap();
        let mut r = b.reader();
        let mut buf = [0u8; 3];
        assert!(r.read_exact(&mut buf).is_ok());
        assert_eq!(&buf, b"abc");
        assert!(r.read_exact(&mut buf).is_ok());
        assert_eq!(&buf, b"def");
        assert_eq!(r.read_exact(&mut buf), Err(ReadError::InsufficientData));
    }

    #[test]
    fn test_reader_empty() {
        let b = ZBytes::new();
        let mut r = b.reader();
        assert_eq!(r.remaining(), 0);
        assert_eq!(r.read_byte(), Err(ReadError::InsufficientData));
        let mut buf = [0u8; 1];
        assert_eq!(r.read_exact(&mut buf), Err(ReadError::InsufficientData));
    }

    #[test]
    fn test_reader_remaining() {
        let b = ZBytes::from_slice(b"hello").unwrap();
        let mut r = b.reader();
        assert_eq!(r.remaining(), 5);
        let _ = r.read_byte();
        assert_eq!(r.remaining(), 4);
        let _ = r.read_byte();
        assert_eq!(r.remaining(), 3);
    }

    #[test]
    fn test_reader_remaining_slice() {
        let b = ZBytes::from_slice(b"hello").unwrap();
        let mut r = b.reader();
        assert_eq!(r.remaining_slice(), b"hello");
        let _ = r.read_byte();
        assert_eq!(r.remaining_slice(), b"ello");
    }

    #[test]
    fn test_reader_position() {
        let b = ZBytes::from_slice(b"abc").unwrap();
        let mut r = b.reader();
        assert_eq!(r.position(), 0);
        let _ = r.read_byte();
        assert_eq!(r.position(), 1);
    }

    #[test]
    fn test_reader_reset() {
        let b = ZBytes::from_slice(b"abc").unwrap();
        let mut r = b.reader();
        let _ = r.read_byte();
        let _ = r.read_byte();
        assert_eq!(r.position(), 2);
        r.reset();
        assert_eq!(r.position(), 0);
        assert_eq!(r.read_byte(), Ok(b'a'));
    }

    #[test]
    fn test_reader_skip() {
        let b = ZBytes::from_slice(b"abcde").unwrap();
        let mut r = b.reader();
        assert!(r.skip(2).is_ok());
        assert_eq!(r.read_byte(), Ok(b'c'));
        assert!(r.skip(2).is_ok());
        assert_eq!(r.read_byte(), Err(ReadError::InsufficientData));
    }

    #[test]
    fn test_reader_skip_past_end() {
        let b = ZBytes::from_slice(b"abc").unwrap();
        let mut r = b.reader();
        assert_eq!(r.skip(10), Err(ReadError::InsufficientData));
    }

    #[test]
    fn test_reader_into_inner() {
        let b = ZBytes::from_slice(b"hello").unwrap();
        let r = b.reader();
        assert_eq!(r.into_inner(), b"hello");
    }

    // --- ZBytesWriter tests ---

    #[test]
    fn test_writer_write_byte() {
        let mut w = ZBytesWriter::new();
        assert!(w.write_byte(b'a').is_ok());
        assert!(w.write_byte(b'b').is_ok());
        assert!(w.write_byte(b'c').is_ok());
        let b = w.finish();
        assert_eq!(b.as_slice(), b"abc");
    }

    #[test]
    fn test_writer_write_slice() {
        let mut w = ZBytesWriter::new();
        assert!(w.write_slice(b"hello ").is_ok());
        assert!(w.write_slice(b"world").is_ok());
        let b = w.finish();
        assert_eq!(b.as_slice(), b"hello world");
    }

    #[test]
    fn test_writer_empty() {
        let w = ZBytesWriter::new();
        assert!(w.is_empty());
        assert_eq!(w.len(), 0);
        let b = w.finish();
        assert!(b.is_empty());
    }

    #[test]
    fn test_writer_capacity_exceeded_byte() {
        let mut w = ZBytesWriterGeneric::<3>::new();
        assert!(w.write_byte(b'a').is_ok());
        assert!(w.write_byte(b'b').is_ok());
        assert!(w.write_byte(b'c').is_ok());
        assert_eq!(w.write_byte(b'd'), Err(WriteError::CapacityExceeded));
    }

    #[test]
    fn test_writer_capacity_exceeded_slice() {
        let mut w = ZBytesWriterGeneric::<3>::new();
        assert!(w.write_slice(b"ab").is_ok());
        assert_eq!(w.write_slice(b"cd"), Err(WriteError::CapacityExceeded));
    }

    #[test]
    fn test_writer_remaining_capacity() {
        let mut w = ZBytesWriterGeneric::<10>::new();
        assert_eq!(w.remaining_capacity(), 10);
        assert!(w.write_slice(b"abc").is_ok());
        assert_eq!(w.remaining_capacity(), 7);
    }

    #[test]
    fn test_writer_as_slice() {
        let mut w = ZBytesWriter::new();
        w.write_slice(b"test").unwrap();
        assert_eq!(w.as_slice(), b"test");
    }

    #[test]
    fn test_writer_clear() {
        let mut w = ZBytesWriter::new();
        w.write_slice(b"hello").unwrap();
        assert!(!w.is_empty());
        w.clear();
        assert!(w.is_empty());
        assert_eq!(w.as_slice(), b"");
    }

    #[test]
    fn test_writer_into_zbytes_via_from() {
        let mut w = ZBytesWriter::new();
        w.write_slice(b"from writer").unwrap();
        let b: ZBytes = w.into();
        assert_eq!(b.as_slice(), b"from writer");
    }

    // --- Writer+Reader integration ---

    #[test]
    fn test_write_then_read_roundtrip() {
        let mut w = ZBytesWriter::new();
        w.write_slice(b"hello").unwrap();
        let b = w.finish();

        let mut r = b.reader();
        let mut buf = [0u8; 5];
        r.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"hello");
    }

    #[test]
    fn test_generic_type_alias() {
        let b = ZBytesGeneric::<DEFAULT_CAPACITY>::from_slice(b"alias test").unwrap();
        assert_eq!(b.as_slice(), b"alias test");
    }
}
