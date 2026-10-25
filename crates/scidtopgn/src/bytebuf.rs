//! [`ByteBuffer`], which reads and writes the bytes of a game record.

use crate::error::{CorruptGameKind, Error, Result};

/// A buffer for reading and writing a game record's bytes.
///
/// Reads start at a read position and stop at the end of the data written so
/// far. A buffer made with [`from_slice`](Self::from_slice) or
/// [`provide_external`](Self::provide_external) holds existing data to read
/// and can't be written. Multi-byte numbers are big-endian, as SCID stores
/// them.
///
/// # Examples
///
/// ```
/// use scidtopgn::ByteBuffer;
///
/// let mut buf = ByteBuffer::new();
/// buf.put_u16(0x1234)?;
/// buf.put_terminated_string("e4")?;
/// assert_eq!(buf.as_slice(), [0x12, 0x34, b'e', b'4', 0]);
///
/// let mut read = ByteBuffer::from_slice(buf.as_slice());
/// assert_eq!(read.get_u16()?, 0x1234);
/// assert_eq!(read.get_terminated_string()?, "e4");
/// # Ok::<(), scidtopgn::Error>(())
/// ```
pub struct ByteBuffer {
    buffer: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    external: bool,
}

impl ByteBuffer {
    /// Makes an empty buffer for writing, with room for 4,096 bytes before
    /// it reallocates.
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    /// Makes an empty buffer for writing, with room for `capacity` bytes
    /// before it reallocates.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            read_pos: 0,
            write_pos: 0,
            external: false,
        }
    }

    /// Makes a read-only buffer holding a copy of `data`.
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            buffer: data.to_vec(),
            read_pos: 0,
            write_pos: data.len(),
            external: true,
        }
    }

    /// Discards the data and makes the buffer writable.
    pub fn empty(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
        self.write_pos = 0;
        self.external = false;
    }

    /// Moves the read position back to the first byte.
    pub fn back_to_start(&mut self) {
        self.read_pos = 0;
    }

    /// Returns the number of bytes of data.
    pub fn len(&self) -> usize {
        self.write_pos
    }

    /// Returns whether the buffer has no data.
    pub fn is_empty(&self) -> bool {
        self.write_pos == 0
    }

    /// Returns the read position: the number of bytes read or skipped.
    pub fn position(&self) -> usize {
        self.read_pos
    }

    /// Returns the data.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.write_pos]
    }

    /// Returns the data, consuming the buffer.
    pub fn into_vec(self) -> Vec<u8> {
        let mut v = self.buffer;
        v.truncate(self.write_pos);
        v
    }

    /// The error for reading past the end of the data.
    fn unexpected_end(&self) -> Error {
        Error::CorruptGame {
            offset: self.read_pos,
            kind: CorruptGameKind::UnexpectedEnd,
        }
    }

    /// Reads a byte.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends first.
    pub fn get_byte(&mut self) -> Result<u8> {
        if self.read_pos >= self.write_pos {
            return Err(self.unexpected_end());
        }
        let b = self.buffer[self.read_pos];
        self.read_pos += 1;
        Ok(b)
    }

    /// Writes a byte.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BufferFull`] if the buffer was made from existing
    /// data, which is read-only.
    pub fn put_byte(&mut self, value: u8) -> Result<()> {
        if self.external {
            return Err(Error::BufferFull);
        }
        self.buffer.push(value);
        self.write_pos += 1;
        Ok(())
    }

    /// Reads a big-endian `u16`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends first.
    pub fn get_u16(&mut self) -> Result<u16> {
        if self.read_pos + 2 > self.write_pos {
            return Err(self.unexpected_end());
        }
        let high = self.buffer[self.read_pos] as u16;
        let low = self.buffer[self.read_pos + 1] as u16;
        self.read_pos += 2;
        Ok((high << 8) | low)
    }

    /// Writes a big-endian `u16`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BufferFull`] if the buffer was made from existing
    /// data, which is read-only.
    pub fn put_u16(&mut self, value: u16) -> Result<()> {
        if self.external {
            return Err(Error::BufferFull);
        }
        let high = ((value >> 8) & 0xFF) as u8;
        let low = (value & 0xFF) as u8;
        self.buffer.push(high);
        self.buffer.push(low);
        self.write_pos += 2;
        Ok(())
    }

    /// Moves the read position past `count` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends first.
    pub fn skip(&mut self, count: usize) -> Result<()> {
        if self.read_pos + count > self.write_pos {
            return Err(self.unexpected_end());
        }
        self.read_pos += count;
        Ok(())
    }

    /// Reads `length` bytes as a string. Bytes that aren't valid UTF-8
    /// become U+FFFD.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends first.
    pub fn get_fixed_string(&mut self, length: usize) -> Result<String> {
        if self.read_pos + length > self.write_pos {
            return Err(self.unexpected_end());
        }
        let s = String::from_utf8_lossy(&self.buffer[self.read_pos..self.read_pos + length]);
        self.read_pos += length;
        Ok(s.to_string())
    }

    /// Writes exactly `length` bytes: the start of `s`, padded with NULs if
    /// `s` is shorter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BufferFull`] if the buffer was made from existing
    /// data, which is read-only.
    pub fn put_fixed_string(&mut self, s: &str, length: usize) -> Result<()> {
        if self.external {
            return Err(Error::BufferFull);
        }
        let bytes = s.as_bytes();
        for i in 0..length {
            if i < bytes.len() {
                self.buffer.push(bytes[i]);
            } else {
                self.buffer.push(0);
            }
        }
        self.write_pos += length;
        Ok(())
    }

    /// Reads a string up to its terminating NUL, and the NUL. Bytes that
    /// aren't valid UTF-8 become U+FFFD.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends before the NUL.
    pub fn get_terminated_string(&mut self) -> Result<String> {
        let start = self.read_pos;
        let data = &self.buffer[start..self.write_pos];
        let Some(length) = data.iter().position(|&b| b == 0) else {
            self.read_pos = self.write_pos;
            return Err(self.unexpected_end());
        };
        let s = String::from_utf8_lossy(&data[..length]).to_string();
        self.read_pos += length + 1;
        Ok(s)
    }

    /// Writes `s` followed by a NUL.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BufferFull`] if the buffer was made from existing
    /// data, which is read-only.
    pub fn put_terminated_string(&mut self, s: &str) -> Result<()> {
        if self.external {
            return Err(Error::BufferFull);
        }
        for b in s.bytes() {
            self.buffer.push(b);
        }
        self.buffer.push(0);
        self.write_pos += s.len() + 1;
        Ok(())
    }

    /// Replaces the data with a copy of `data`, to read from the start. The
    /// buffer becomes read-only.
    pub fn provide_external(&mut self, data: &[u8]) {
        self.buffer = data.to_vec();
        self.read_pos = 0;
        self.write_pos = data.len();
        self.external = true;
    }

    /// Returns the number of bytes of data, like [`len`](Self::len).
    pub fn byte_count(&self) -> usize {
        self.write_pos
    }
}

impl Default for ByteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get_byte() {
        let mut bb = ByteBuffer::new();
        bb.put_byte(0x42).unwrap();
        bb.back_to_start();
        assert_eq!(bb.get_byte().unwrap(), 0x42);
    }

    #[test]
    fn test_put_get_u16_big_endian() {
        let mut bb = ByteBuffer::new();
        bb.put_u16(0x1234).unwrap();
        assert_eq!(bb.as_slice(), &[0x12, 0x34]);
        bb.back_to_start();
        assert_eq!(bb.get_u16().unwrap(), 0x1234);
    }

    #[test]
    fn test_terminated_string() {
        let mut bb = ByteBuffer::new();
        bb.put_terminated_string("hello").unwrap();
        assert_eq!(bb.as_slice(), &[b'h', b'e', b'l', b'l', b'o', 0]);
        bb.back_to_start();
        assert_eq!(bb.get_terminated_string().unwrap(), "hello");
    }

    #[test]
    fn test_from_slice() {
        let data = [0x01, 0x02, 0x03, 0x00, 0x04];
        let mut bb = ByteBuffer::from_slice(&data);
        assert_eq!(bb.get_byte().unwrap(), 0x01);
        assert_eq!(bb.get_u16().unwrap(), 0x0203);
    }

    #[test]
    fn test_fixed_string_is_padded_with_nuls() {
        let mut bb = ByteBuffer::default();
        assert!(bb.is_empty());
        bb.put_fixed_string("ab", 4).unwrap();
        assert!(!bb.is_empty());
        bb.back_to_start();
        assert_eq!(bb.get_fixed_string(4).unwrap(), "ab\0\0");
        assert_eq!(bb.into_vec(), [b'a', b'b', 0, 0]);
    }

    #[test]
    fn test_reads_past_the_end_give_their_offset() {
        let mut bb = ByteBuffer::from_slice(&[1, 2, 3]);
        bb.skip(2).unwrap();
        let unexpected_end = |result: Result<()>| {
            assert!(
                matches!(
                    result,
                    Err(Error::CorruptGame {
                        offset: 2,
                        kind: CorruptGameKind::UnexpectedEnd,
                    })
                ),
                "{result:?}"
            );
        };
        unexpected_end(bb.get_u16().map(drop));
        unexpected_end(bb.skip(2));
        unexpected_end(bb.get_fixed_string(2).map(drop));
        // Failed reads don't move the read position.
        assert_eq!(bb.position(), 2);
        assert_eq!(bb.get_byte().unwrap(), 3);
    }

    #[test]
    fn test_external_data_is_read_only() {
        let mut bb = ByteBuffer::from_slice(&[1]);
        assert!(matches!(bb.put_byte(0), Err(Error::BufferFull)));
        assert!(matches!(bb.put_u16(0), Err(Error::BufferFull)));
        assert!(matches!(
            bb.put_fixed_string("a", 1),
            Err(Error::BufferFull)
        ));
        assert!(matches!(
            bb.put_terminated_string("a"),
            Err(Error::BufferFull)
        ));
        assert_eq!(Error::BufferFull.to_string(), "Buffer full");
        assert_eq!(bb.as_slice(), [1]);

        // Emptying the buffer makes it writable again.
        bb.empty();
        bb.put_byte(2).unwrap();
        assert_eq!(bb.as_slice(), [2]);
    }
}
