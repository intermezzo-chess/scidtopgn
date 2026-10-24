use crate::error::{CorruptGameKind, Error, Result};

pub struct ByteBuffer {
    buffer: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    external: bool,
}

impl ByteBuffer {
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            read_pos: 0,
            write_pos: 0,
            external: false,
        }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            buffer: data.to_vec(),
            read_pos: 0,
            write_pos: data.len(),
            external: true,
        }
    }

    pub fn empty(&mut self) {
        self.buffer.clear();
        self.read_pos = 0;
        self.write_pos = 0;
        self.external = false;
    }

    pub fn back_to_start(&mut self) {
        self.read_pos = 0;
    }

    pub fn len(&self) -> usize {
        self.write_pos
    }

    pub fn is_empty(&self) -> bool {
        self.write_pos == 0
    }

    pub fn position(&self) -> usize {
        self.read_pos
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.write_pos]
    }

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

    pub fn get_byte(&mut self) -> Result<u8> {
        if self.read_pos >= self.write_pos {
            return Err(self.unexpected_end());
        }
        let b = self.buffer[self.read_pos];
        self.read_pos += 1;
        Ok(b)
    }

    pub fn put_byte(&mut self, value: u8) -> Result<()> {
        if self.external {
            return Err(Error::BufferFull);
        }
        self.buffer.push(value);
        self.write_pos += 1;
        Ok(())
    }

    pub fn get_u16(&mut self) -> Result<u16> {
        if self.read_pos + 2 > self.write_pos {
            return Err(self.unexpected_end());
        }
        let high = self.buffer[self.read_pos] as u16;
        let low = self.buffer[self.read_pos + 1] as u16;
        self.read_pos += 2;
        Ok((high << 8) | low)
    }

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

    pub fn skip(&mut self, count: usize) -> Result<()> {
        if self.read_pos + count > self.write_pos {
            return Err(self.unexpected_end());
        }
        self.read_pos += count;
        Ok(())
    }

    pub fn get_fixed_string(&mut self, length: usize) -> Result<String> {
        if self.read_pos + length > self.write_pos {
            return Err(self.unexpected_end());
        }
        let s = String::from_utf8_lossy(&self.buffer[self.read_pos..self.read_pos + length]);
        self.read_pos += length;
        Ok(s.to_string())
    }

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

    pub fn get_terminated_string(&mut self) -> Result<String> {
        let start = self.read_pos;
        while self.read_pos < self.write_pos && self.buffer[self.read_pos] != 0 {
            self.read_pos += 1;
        }
        let s = String::from_utf8_lossy(&self.buffer[start..self.read_pos]).to_string();
        if self.read_pos < self.write_pos {
            self.read_pos += 1;
        }
        Ok(s)
    }

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

    pub fn provide_external(&mut self, data: &[u8]) {
        self.buffer = data.to_vec();
        self.read_pos = 0;
        self.write_pos = data.len();
        self.external = true;
    }

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
}
