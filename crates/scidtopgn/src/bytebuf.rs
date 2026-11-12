//! [`ByteBuffer`], which reads and writes the bytes of a game record.

use crate::error::{CorruptGameKind, Error, Result};

/// A buffer for reading and writing a game record's bytes.
///
/// Reads start at a read position and stop at the end of the data written so
/// far. A buffer made with [`from_slice`](Self::from_slice) or
/// [`provide_external`](Self::provide_external) holds existing data to read.
/// Multi-byte numbers are big-endian, as SCID stores them.
pub(crate) struct ByteBuffer {
    buffer: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
}

impl ByteBuffer {
    /// Makes an empty buffer for writing, with room for 4,096 bytes before
    /// it reallocates.
    pub(crate) fn new() -> Self {
        Self::with_capacity(4096)
    }

    /// Makes an empty buffer for writing, with room for `capacity` bytes
    /// before it reallocates.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            read_pos: 0,
            write_pos: 0,
        }
    }

    /// Makes a buffer holding a copy of `data`, to read from the start.
    pub(crate) fn from_slice(data: &[u8]) -> Self {
        Self {
            buffer: data.to_vec(),
            read_pos: 0,
            write_pos: data.len(),
        }
    }

    /// Moves the read position back to the first byte.
    pub(crate) fn back_to_start(&mut self) {
        self.read_pos = 0;
    }

    /// Returns the number of bytes of data.
    pub(crate) fn len(&self) -> usize {
        self.write_pos
    }

    /// Returns the read position: the number of bytes read or skipped.
    pub(crate) fn position(&self) -> usize {
        self.read_pos
    }

    /// Returns the data.
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.write_pos]
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
    pub(crate) fn get_byte(&mut self) -> Result<u8> {
        if self.read_pos >= self.write_pos {
            return Err(self.unexpected_end());
        }
        let b = self.buffer[self.read_pos];
        self.read_pos += 1;
        Ok(b)
    }

    /// Writes a byte.
    pub(crate) fn put_byte(&mut self, value: u8) {
        self.buffer.push(value);
        self.write_pos += 1;
    }

    /// Moves the read position past `count` bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends first.
    pub(crate) fn skip(&mut self, count: usize) -> Result<()> {
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
    pub(crate) fn get_fixed_string(&mut self, length: usize) -> Result<String> {
        if self.read_pos + length > self.write_pos {
            return Err(self.unexpected_end());
        }
        let s = String::from_utf8_lossy(&self.buffer[self.read_pos..self.read_pos + length]);
        self.read_pos += length;
        Ok(s.to_string())
    }

    /// Writes exactly `length` bytes: the start of `s`, padded with NULs if
    /// `s` is shorter.
    pub(crate) fn put_fixed_string(&mut self, s: &str, length: usize) {
        let bytes = s.as_bytes();
        for i in 0..length {
            if i < bytes.len() {
                self.buffer.push(bytes[i]);
            } else {
                self.buffer.push(0);
            }
        }
        self.write_pos += length;
    }

    /// Reads a string up to its terminating NUL, and the NUL. Bytes that
    /// aren't valid UTF-8 become U+FFFD.
    ///
    /// # Errors
    ///
    /// Returns [`Error::CorruptGame`] if the data ends before the NUL.
    pub(crate) fn get_terminated_string(&mut self) -> Result<String> {
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
    pub(crate) fn put_terminated_string(&mut self, s: &str) {
        for b in s.bytes() {
            self.buffer.push(b);
        }
        self.buffer.push(0);
        self.write_pos += s.len() + 1;
    }

    /// Replaces the data with a copy of `data`, to read from the start.
    pub(crate) fn provide_external(&mut self, data: &[u8]) {
        self.buffer = data.to_vec();
        self.read_pos = 0;
        self.write_pos = data.len();
    }

    /// Returns the number of bytes of data, like [`len`](Self::len).
    pub(crate) fn byte_count(&self) -> usize {
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
        bb.put_byte(0x42);
        bb.back_to_start();
        assert_eq!(bb.get_byte().unwrap(), 0x42);
    }

    #[test]
    fn test_terminated_string() {
        let mut bb = ByteBuffer::new();
        bb.put_terminated_string("hello");
        assert_eq!(bb.as_slice(), &[b'h', b'e', b'l', b'l', b'o', 0]);
        bb.back_to_start();
        assert_eq!(bb.get_terminated_string().unwrap(), "hello");
    }

    #[test]
    fn test_from_slice() {
        let data = [0x01, 0x02, 0x03, 0x00, 0x04];
        let mut bb = ByteBuffer::from_slice(&data);
        assert_eq!(bb.get_byte().unwrap(), 0x01);
        assert_eq!(bb.get_byte().unwrap(), 0x02);
        assert_eq!(bb.get_terminated_string().unwrap(), "\u{3}");
    }

    #[test]
    fn test_fixed_string_is_padded_with_nuls() {
        let mut bb = ByteBuffer::default();
        bb.put_fixed_string("ab", 4);
        assert_eq!(bb.as_slice(), [b'a', b'b', 0, 0]);
        bb.back_to_start();
        assert_eq!(bb.get_fixed_string(4).unwrap(), "ab\0\0");
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
        unexpected_end(bb.skip(2));
        unexpected_end(bb.get_fixed_string(2).map(drop));
        // Failed reads don't move the read position.
        assert_eq!(bb.position(), 2);
        assert_eq!(bb.get_byte().unwrap(), 3);
    }

    #[test]
    fn test_provide_external_reads_replacement_from_the_start() {
        // Longer than, shorter than, and empty compared with the 3 bytes read.
        let replacements: [&[u8]; 3] = [&[10, 11, 12, 13, 14, 15], &[10, 11], &[]];
        for replacement in replacements {
            let mut bb = ByteBuffer::from_slice(&[1, 2, 3, 4, 5]);
            for byte in [1, 2, 3] {
                assert_eq!(bb.get_byte().unwrap(), byte);
            }

            bb.provide_external(replacement);
            assert_eq!(bb.position(), 0);
            assert_eq!(bb.len(), replacement.len());
            assert_eq!(bb.as_slice(), replacement);

            for &byte in replacement {
                assert_eq!(bb.get_byte().unwrap(), byte);
            }
            assert!(bb.get_byte().is_err());
        }
    }
}
