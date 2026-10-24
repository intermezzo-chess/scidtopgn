use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::bytebuf::ByteBuffer;
use crate::error::{DatabaseFile, Error, Result};

pub const GFILE_SUFFIX: &str = ".sg4";
pub const GF_BLOCKSIZE: usize = 131_072;

struct GfBlock {
    block_num: i32,
    dirty: bool,
    length: usize,
    data: Vec<u8>,
}

impl GfBlock {
    fn new() -> Self {
        Self {
            block_num: -1,
            dirty: false,
            length: 0,
            data: vec![0u8; GF_BLOCKSIZE],
        }
    }

    fn clear(&mut self) {
        self.length = 0;
        self.data.fill(0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    ReadOnly,
    WriteOnly,
    Both,
    None,
}

pub struct GFile {
    handle: Option<File>,
    file_mode: FileMode,
    offset: u64,
    num_blocks: u32,
    last_block_size: usize,
    current_block: GfBlock,
}

impl GFile {
    pub fn new() -> Self {
        Self {
            handle: None,
            file_mode: FileMode::None,
            offset: 0,
            num_blocks: 0,
            last_block_size: 0,
            current_block: GfBlock::new(),
        }
    }

    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let sg4_path = path.as_ref().with_extension("sg4");

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&sg4_path)
            .map_err(|e| Error::from(e).in_file(DatabaseFile::Game))?;

        Ok(Self {
            handle: Some(file),
            file_mode: FileMode::WriteOnly,
            offset: 0,
            num_blocks: 0,
            last_block_size: 0,
            current_block: GfBlock::new(),
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let sg4_path = path.as_ref().with_extension("sg4");
        let open = || -> Result<(File, u64)> {
            let file = OpenOptions::new().read(true).write(true).open(&sg4_path)?;
            let size = file.metadata()?.len();
            Ok((file, size))
        };
        let (file, file_size) = open().map_err(|e| e.in_file(DatabaseFile::Game))?;
        let file_size = file_size as usize;

        let num_blocks = if file_size == 0 {
            0
        } else {
            file_size.div_ceil(GF_BLOCKSIZE)
        };

        let last_block_size = if num_blocks == 0 {
            0
        } else {
            let rem = file_size % GF_BLOCKSIZE;
            if rem == 0 {
                GF_BLOCKSIZE
            } else {
                rem
            }
        };

        Ok(Self {
            handle: Some(file),
            file_mode: FileMode::Both,
            offset: 0,
            num_blocks: num_blocks as u32,
            last_block_size,
            current_block: GfBlock::new(),
        })
    }

    pub fn file_size(&self) -> u64 {
        if self.num_blocks == 0 {
            return 0;
        }
        ((self.num_blocks - 1) as u64) * (GF_BLOCKSIZE as u64) + (self.last_block_size as u64)
    }

    pub fn close(&mut self) -> Result<()> {
        if self.current_block.dirty && self.file_mode != FileMode::ReadOnly {
            self.flush_block()?;
        }
        if let Some(ref mut file) = self.handle {
            file.flush()?;
        }
        self.handle = None;
        self.file_mode = FileMode::None;
        Ok(())
    }

    fn flush_block(&mut self) -> Result<()> {
        let handle = match &mut self.handle {
            Some(h) => h,
            None => return Err(Error::FileNotOpen),
        };

        if self.file_mode == FileMode::ReadOnly {
            return Err(Error::ReadOnly);
        }

        if !self.current_block.dirty {
            return Ok(());
        }

        if self.current_block.block_num < 0 {
            return Ok(());
        }

        let file_pos = (self.current_block.block_num as u64) * (GF_BLOCKSIZE as u64);

        if self.offset != file_pos {
            handle.seek(SeekFrom::Start(file_pos))?;
            self.offset = file_pos;
        }

        let num_bytes = if self.current_block.block_num == (self.num_blocks as i32) - 1 {
            self.current_block.length
        } else {
            GF_BLOCKSIZE
        };

        handle.write_all(&self.current_block.data[..num_bytes])?;

        if self.file_mode == FileMode::Both {
            handle.flush()?;
        }

        self.offset += num_bytes as u64;
        self.current_block.dirty = false;

        Ok(())
    }

    fn fetch_block(&mut self, block_num: u32) -> Result<()> {
        if self.current_block.dirty && self.file_mode != FileMode::ReadOnly {
            self.flush_block()?;
        }

        let Some(handle) = self.handle.as_mut() else {
            return Err(Error::FileNotOpen);
        };

        let file_pos = (block_num as u64) * (GF_BLOCKSIZE as u64);

        if self.offset != file_pos {
            handle.seek(SeekFrom::Start(file_pos))?;
            self.offset = file_pos;
        }

        let num_bytes = if block_num == self.num_blocks - 1 {
            self.last_block_size
        } else {
            GF_BLOCKSIZE
        };

        if num_bytes > 0 {
            handle.read_exact(&mut self.current_block.data[..num_bytes])?;
        }

        self.offset += num_bytes as u64;
        self.current_block.dirty = false;
        self.current_block.block_num = block_num as i32;
        self.current_block.length = num_bytes;

        Ok(())
    }

    /// Reads the `length` bytes of game data at `offset` into `buf`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameOutOfRange`] if the data extends past the end of
    /// the file, and [`Error::GameSpansBlocks`] if it crosses a block
    /// boundary.
    pub fn read_game(&mut self, buf: &mut ByteBuffer, offset: u32, length: u32) -> Result<()> {
        if u64::from(offset) + u64::from(length) > self.file_size() {
            return Err(Error::GameOutOfRange { offset, length });
        }
        if length == 0 {
            buf.provide_external(&[]);
            return Ok(());
        }

        let block_num = offset as usize / GF_BLOCKSIZE;
        let end_block_num = (offset as usize + length as usize - 1) / GF_BLOCKSIZE;
        if end_block_num != block_num {
            return Err(Error::GameSpansBlocks { offset, length });
        }

        if self.current_block.block_num != block_num as i32 {
            self.fetch_block(block_num as u32)?;
        }

        let start = (offset as usize) % GF_BLOCKSIZE;
        let game_data = &self.current_block.data[start..start + length as usize];

        buf.provide_external(game_data);

        Ok(())
    }

    pub fn read_game_data(&mut self, offset: u32, length: u16) -> Result<Vec<u8>> {
        let mut buf = ByteBuffer::new();
        self.read_game(&mut buf, offset, length as u32)?;
        Ok(buf.as_slice().to_vec())
    }

    /// Appends a game record and returns its offset.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameTooLarge`] if the record is longer than a block.
    pub fn add_game(&mut self, buf: &ByteBuffer) -> Result<u32> {
        if self.handle.is_none() {
            return Err(Error::FileNotOpen);
        }

        if self.file_mode == FileMode::ReadOnly {
            return Err(Error::ReadOnly);
        }

        let data_len = buf.byte_count();
        if data_len > GF_BLOCKSIZE {
            return Err(Error::GameTooLarge {
                length: data_len,
                max: GF_BLOCKSIZE,
            });
        }

        if self.num_blocks == 0 {
            self.current_block.block_num = 0;
            self.current_block.clear();
            self.num_blocks = 1;
        } else {
            if self.current_block.block_num != (self.num_blocks - 1) as i32 {
                self.fetch_block(self.num_blocks - 1)?;
            }

            if self.last_block_size + data_len > GF_BLOCKSIZE {
                self.flush_block()?;
                self.num_blocks += 1;
                self.current_block.block_num = self.num_blocks as i32 - 1;
                self.current_block.clear();
            }
        }

        let game_offset = (self.current_block.block_num as u32) * (GF_BLOCKSIZE as u32)
            + (self.current_block.length as u32);

        self.current_block.data[self.current_block.length..self.current_block.length + data_len]
            .copy_from_slice(buf.as_slice());

        self.current_block.length += data_len;
        self.last_block_size = self.current_block.length;
        self.current_block.dirty = true;

        Ok(game_offset)
    }

    pub fn add_game_data(&mut self, data: &[u8]) -> Result<u32> {
        let buf = ByteBuffer::from_slice(data);
        self.add_game(&buf)
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.current_block.dirty {
            self.flush_block()?;
        }
        if let Some(ref mut file) = self.handle {
            file.flush()?;
        }
        Ok(())
    }
}

impl Default for GFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GFile {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_gfile_create_and_open() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test");

        {
            let mut gf = GFile::create(&path).unwrap();
            let buf = ByteBuffer::from_slice(&[1, 2, 3, 4, 5]);
            let offset = gf.add_game(&buf).unwrap();
            assert_eq!(offset, 0);
            gf.flush().unwrap();
        }

        let gf = GFile::open(&path).unwrap();
        assert_eq!(gf.num_blocks, 1);
    }

    #[test]
    fn test_gfile_write_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test");

        let test_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let offset;
        {
            let mut gf = GFile::create(&path).unwrap();
            let buf = ByteBuffer::from_slice(&test_data);
            offset = gf.add_game(&buf).unwrap();
            gf.flush().unwrap();
        }

        {
            let mut gf = GFile::open(&path).unwrap();
            let mut buf = ByteBuffer::new();
            gf.read_game(&mut buf, offset, test_data.len() as u32)
                .unwrap();

            assert_eq!(buf.as_slice(), test_data.as_slice());
        }
    }

    #[test]
    fn test_gfile_multiple_games() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test");

        let data1 = vec![1, 2, 3];
        let data2 = vec![4, 5, 6, 7];
        let data3 = vec![8, 9];

        let (offset1, offset2, offset3);
        {
            let mut gf = GFile::create(&path).unwrap();

            let buf1 = ByteBuffer::from_slice(&data1);
            offset1 = gf.add_game(&buf1).unwrap();

            let buf2 = ByteBuffer::from_slice(&data2);
            offset2 = gf.add_game(&buf2).unwrap();

            let buf3 = ByteBuffer::from_slice(&data3);
            offset3 = gf.add_game(&buf3).unwrap();

            gf.flush().unwrap();
        }

        {
            let mut gf = GFile::open(&path).unwrap();

            let mut buf = ByteBuffer::new();
            gf.read_game(&mut buf, offset1, data1.len() as u32).unwrap();
            assert_eq!(buf.as_slice(), data1.as_slice());

            buf.empty();
            gf.read_game(&mut buf, offset2, data2.len() as u32).unwrap();
            assert_eq!(buf.as_slice(), data2.as_slice());

            buf.empty();
            gf.read_game(&mut buf, offset3, data3.len() as u32).unwrap();
            assert_eq!(buf.as_slice(), data3.as_slice());
        }
    }
}
