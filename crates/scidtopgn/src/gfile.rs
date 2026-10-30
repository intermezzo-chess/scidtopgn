//! The `.sg4` game file, which stores each game's encoded record.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::bytebuf::ByteBuffer;
use crate::error::{DatabaseFile, Error, Result};

/// The extension of a game file.
pub const GFILE_SUFFIX: &str = ".sg4";
/// The size of a block of the game file, in bytes. A record is stored in
/// the block it starts in unless that block is full.
pub const GF_BLOCKSIZE: usize = 131_072;
/// The longest game record a database can store. The index stores a
/// record's length in 17 bits.
pub(crate) const MAX_GAME_LENGTH: usize = GF_BLOCKSIZE - 1;

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

/// How a [`GFile`] has its file open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// Open for reading.
    ReadOnly,
    /// Open for writing.
    WriteOnly,
    /// Open for reading and writing.
    Both,
    /// Not open.
    None,
}

/// A `.sg4` game file, read and written a block at a time.
///
/// Records are found by the offset and length in their index entry. Most
/// programs use [`Database`](crate::Database), which keeps the game file in
/// step with the index.
///
/// Dropping a `GFile` writes its pending block and ignores any error;
/// call [`close`](Self::close) or [`flush`](Self::flush) to see it.
pub struct GFile {
    handle: Option<File>,
    file_mode: FileMode,
    offset: u64,
    num_blocks: u32,
    last_block_size: usize,
    current_block: GfBlock,
}

impl GFile {
    /// Makes a `GFile` with no file open.
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

    /// Creates the game file of the database at `path`, given without an
    /// extension, replacing any file already there, and opens it for
    /// reading and writing, so games read back before it's closed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] for [`DatabaseFile::Game`] if the file can't
    /// be created.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let sg4_path = path.as_ref().with_extension("sg4");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&sg4_path)
            .map_err(|e| Error::from(e).in_file(DatabaseFile::Game))?;

        Ok(Self::from_empty_file(file))
    }

    /// Makes a game file from `file`, which is empty and open for reading
    /// and writing.
    pub(crate) fn from_empty_file(file: File) -> Self {
        Self {
            handle: Some(file),
            file_mode: FileMode::Both,
            offset: 0,
            num_blocks: 0,
            last_block_size: 0,
            current_block: GfBlock::new(),
        }
    }

    /// Opens the game file of the database at `path`, given without an
    /// extension, for reading and writing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] for [`DatabaseFile::Game`] if the file can't
    /// be opened.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_file(path.as_ref(), FileMode::Both)
    }

    /// Opens the game file of the database at `path`, given without an
    /// extension, for reading only, so the file needn't be writable. Adding
    /// a game returns [`Error::ReadOnly`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] for [`DatabaseFile::Game`] if the file can't
    /// be opened.
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_file(path.as_ref(), FileMode::ReadOnly)
    }

    /// Opens the game file in `file_mode`, which is
    /// [`ReadOnly`](FileMode::ReadOnly) or [`Both`](FileMode::Both).
    pub(crate) fn open_file(path: &Path, file_mode: FileMode) -> Result<Self> {
        let sg4_path = path.with_extension("sg4");
        let open = || -> Result<(File, u64)> {
            let file = OpenOptions::new()
                .read(true)
                .write(file_mode == FileMode::Both)
                .open(&sg4_path)?;
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
            file_mode,
            offset: 0,
            num_blocks: num_blocks as u32,
            last_block_size,
            current_block: GfBlock::new(),
        })
    }

    /// Returns how the file is open.
    pub(crate) fn file_mode(&self) -> FileMode {
        self.file_mode
    }

    /// Returns the length of the game data in bytes, including records not
    /// yet written to the file.
    pub fn file_size(&self) -> u64 {
        if self.num_blocks == 0 {
            return 0;
        }
        ((self.num_blocks - 1) as u64) * (GF_BLOCKSIZE as u64) + (self.last_block_size as u64)
    }

    /// Writes the pending block, if any, and closes the file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the block can't be written.
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

    /// Drops the pending block, cuts the file to `len` bytes, and closes
    /// it, for [`Database::discard`](crate::Database::discard).
    pub(crate) fn discard_after(&mut self, len: u64) -> Result<()> {
        self.current_block.dirty = false;
        self.file_mode = FileMode::None;
        if let Some(file) = self.handle.take() {
            file.set_len(len)?;
        }
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

    /// Reads the `length` bytes of game data at `offset` into `buf`. Like
    /// SCID, it reads a record that crosses a block boundary.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameOutOfRange`] if the data extends past the end of
    /// the file, [`Error::FileNotOpen`] if the file is closed, and
    /// [`Error::Io`] if it can't be read.
    pub fn read_game(&mut self, buf: &mut ByteBuffer, offset: u32, length: u32) -> Result<()> {
        if u64::from(offset) + u64::from(length) > self.file_size() {
            return Err(Error::GameOutOfRange { offset, length });
        }

        let mut pos = offset as usize;
        let end = pos + length as usize;
        let mut data = Vec::with_capacity(length as usize);
        while pos < end {
            let block_num = pos / GF_BLOCKSIZE;
            if self.current_block.block_num != block_num as i32 {
                self.fetch_block(block_num as u32)?;
            }
            let start = pos % GF_BLOCKSIZE;
            let count = (GF_BLOCKSIZE - start).min(end - pos);
            data.extend_from_slice(&self.current_block.data[start..start + count]);
            pos += count;
        }

        buf.provide_external(&data);

        Ok(())
    }

    /// Returns the `length` bytes of game data at `offset`.
    ///
    /// # Errors
    ///
    /// The same as [`read_game`](Self::read_game).
    pub fn read_game_data(&mut self, offset: u32, length: u32) -> Result<Vec<u8>> {
        let mut buf = ByteBuffer::new();
        self.read_game(&mut buf, offset, length)?;
        Ok(buf.as_slice().to_vec())
    }

    /// Appends a game record and returns its offset.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameTooLarge`] if the record is longer than
    /// 131,071 bytes, the most the index can record,
    /// [`Error::GameFileFull`] if it would start past the largest offset the
    /// index can record, [`Error::FileNotOpen`] if the file is closed,
    /// [`Error::ReadOnly`] if it is open only for reading, and [`Error::Io`]
    /// if a full block can't be written.
    pub fn add_game(&mut self, buf: &ByteBuffer) -> Result<u32> {
        if self.handle.is_none() {
            return Err(Error::FileNotOpen);
        }

        if self.file_mode == FileMode::ReadOnly {
            return Err(Error::ReadOnly);
        }

        let data_len = buf.byte_count();
        if data_len > MAX_GAME_LENGTH {
            return Err(Error::GameTooLarge {
                length: data_len,
                max: MAX_GAME_LENGTH,
            });
        }

        // The index stores a record's offset in 32 bits, so a record can't
        // start past it. The offset doesn't depend on the current block.
        let offset = if self.num_blocks > 0 && self.last_block_size + data_len <= GF_BLOCKSIZE {
            u64::from(self.num_blocks - 1) * GF_BLOCKSIZE as u64 + self.last_block_size as u64
        } else {
            u64::from(self.num_blocks) * GF_BLOCKSIZE as u64
        };
        if offset > u64::from(u32::MAX) {
            return Err(Error::GameFileFull);
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

    /// Appends the game record `data` and returns its offset.
    ///
    /// # Errors
    ///
    /// The same as [`add_game`](Self::add_game).
    pub fn add_game_data(&mut self, data: &[u8]) -> Result<u32> {
        let buf = ByteBuffer::from_slice(data);
        self.add_game(&buf)
    }

    /// Writes the pending block, if any, and flushes the file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadOnly`] if the file is open only for reading, and
    /// [`Error::Io`] if it can't be written.
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

    #[test]
    fn test_gfile_without_a_file() {
        let mut gf = GFile::default();
        assert_eq!(gf.file_size(), 0);
        assert!(matches!(
            gf.add_game_data(&[1, 2, 3]),
            Err(Error::FileNotOpen)
        ));
        assert_eq!(Error::FileNotOpen.to_string(), "File not open");
        gf.flush().unwrap();
        gf.close().unwrap();
    }

    #[test]
    fn test_gfile_open_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("empty");
        std::fs::write(path.with_extension("sg4"), []).unwrap();

        let mut gf = GFile::open(&path).unwrap();
        assert_eq!(gf.file_size(), 0);
        assert_eq!(gf.add_game_data(&[1, 2, 3]).unwrap(), 0);
        gf.close().unwrap();
        assert_eq!(
            std::fs::read(path.with_extension("sg4")).unwrap(),
            [1, 2, 3]
        );
    }

    #[test]
    fn test_gfile_open_file_of_one_full_block() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("full");
        std::fs::write(path.with_extension("sg4"), vec![7; GF_BLOCKSIZE]).unwrap();

        let mut gf = GFile::open(&path).unwrap();
        assert_eq!(gf.file_size(), GF_BLOCKSIZE as u64);
        let last = (GF_BLOCKSIZE - 1) as u32;
        assert_eq!(gf.read_game_data(last, 1).unwrap(), [7]);
        // A game added after a full block starts the next one.
        assert_eq!(gf.add_game_data(&[1]).unwrap(), GF_BLOCKSIZE as u32);
    }

    #[test]
    fn test_gfile_close_writes_the_block() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("closed");

        let mut gf = GFile::create(&path).unwrap();
        gf.add_game_data(&[1, 2, 3]).unwrap();
        gf.close().unwrap();
        // Closing again does nothing.
        gf.close().unwrap();
        assert!(matches!(gf.add_game_data(&[4]), Err(Error::FileNotOpen)));

        let mut gf = GFile::open(&path).unwrap();
        assert_eq!(gf.read_game_data(0, 3).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn test_gfile_open_read_only() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("read_only");
        std::fs::write(path.with_extension("sg4"), [1, 2, 3]).unwrap();

        let mut gf = GFile::open_read_only(&path).unwrap();
        assert_eq!(gf.file_mode(), FileMode::ReadOnly);
        assert_eq!(gf.read_game_data(0, 3).unwrap(), [1, 2, 3]);
        assert!(matches!(gf.add_game_data(&[4]), Err(Error::ReadOnly)));
        assert_eq!(gf.file_size(), 3);
        gf.flush().unwrap();
        gf.close().unwrap();

        assert_eq!(
            std::fs::read(path.with_extension("sg4")).unwrap(),
            [1, 2, 3]
        );
    }

    #[test]
    fn test_gfile_read_after_close() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("closed");
        std::fs::write(path.with_extension("sg4"), [1, 2, 3]).unwrap();

        let mut gf = GFile::open(&path).unwrap();
        gf.close().unwrap();
        let mut buf = ByteBuffer::new();
        assert!(matches!(
            gf.read_game(&mut buf, 0, 3),
            Err(Error::FileNotOpen)
        ));
    }

    #[test]
    fn test_gfile_add_and_read_across_blocks_after_open() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("blocks");
        // A last block with room for 100 more bytes.
        std::fs::write(path.with_extension("sg4"), vec![7; GF_BLOCKSIZE - 100]).unwrap();

        let mut gf = GFile::open(&path).unwrap();
        // 200 bytes don't fit, so they start the next block.
        let second = gf.add_game_data(&[8; 200]).unwrap();
        assert_eq!(second, GF_BLOCKSIZE as u32);
        // Reading the first block writes the second one first.
        assert_eq!(gf.read_game_data(0, 2).unwrap(), [7, 7]);
        assert_eq!(gf.read_game_data(second, 200).unwrap(), [8; 200]);
        gf.close().unwrap();

        let size = std::fs::metadata(path.with_extension("sg4")).unwrap().len();
        assert_eq!(size, GF_BLOCKSIZE as u64 + 200);
        let mut gf = GFile::open(&path).unwrap();
        assert_eq!(gf.read_game_data(second, 200).unwrap(), [8; 200]);
        let end = (GF_BLOCKSIZE - 101) as u32;
        assert_eq!(gf.read_game_data(end, 1).unwrap(), [7]);
    }

    #[test]
    fn test_gfile_read_across_blocks() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("blocks");
        // Two full blocks and a partial one, with no two neighbors alike.
        let data: Vec<u8> = (0..2 * GF_BLOCKSIZE + 10)
            .map(|i| (i % 251) as u8)
            .collect();
        std::fs::write(path.with_extension("sg4"), &data).unwrap();
        let read = |offset: usize, length: usize| data[offset..offset + length].to_vec();

        let mut gf = GFile::open(&path).unwrap();
        let block = GF_BLOCKSIZE;
        // Ends at the end of a block.
        assert_eq!(
            gf.read_game_data(block as u32 - 2, 2).unwrap(),
            read(block - 2, 2)
        );
        // Fills a block.
        assert_eq!(
            gf.read_game_data(block as u32, block as u32).unwrap(),
            read(block, block)
        );
        // Crosses one boundary.
        assert_eq!(
            gf.read_game_data(block as u32 - 2, 4).unwrap(),
            read(block - 2, 4)
        );
        // Crosses two, into the last, partial block.
        let length = block + 3;
        assert_eq!(
            gf.read_game_data(block as u32 - 1, length as u32).unwrap(),
            read(block - 1, length)
        );
        assert!(gf.read_game_data(block as u32, 0).unwrap().is_empty());
        assert!(matches!(
            gf.read_game_data(2 * block as u32 + 8, 3),
            Err(Error::GameOutOfRange { .. })
        ));
    }

    #[test]
    fn test_gfile_read_across_blocks_before_writing() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("blocks");
        // A last block with room for 100 more bytes.
        std::fs::write(path.with_extension("sg4"), vec![7; GF_BLOCKSIZE - 100]).unwrap();

        let mut gf = GFile::open(&path).unwrap();
        // Starts the next block, which isn't written yet.
        gf.add_game_data(&[8; 200]).unwrap();
        // The last game, the padding after it, and the start of the new game.
        let mut expected = vec![7];
        expected.extend([0; 100]);
        expected.extend([8, 8]);
        let offset = (GF_BLOCKSIZE - 101) as u32;
        assert_eq!(gf.read_game_data(offset, 103).unwrap(), expected);
    }

    #[test]
    fn test_gfile_create_reads_back_written_blocks() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("created");
        let block = GF_BLOCKSIZE as u32;
        let file_len = || std::fs::metadata(path.with_extension("sg4")).unwrap().len();

        let mut gf = GFile::create(&path).unwrap();
        // The first two fill block 0 exactly, and each of the others starts
        // a new block.
        let records = [
            vec![1; MAX_GAME_LENGTH],
            vec![2; 1],
            vec![3; 100],
            vec![4; MAX_GAME_LENGTH],
            vec![5; 10],
        ];
        let offsets: Vec<u32> = records
            .iter()
            .map(|record| gf.add_game_data(record).unwrap())
            .collect();
        assert_eq!(offsets, [0, block - 1, block, 2 * block, 3 * block]);

        // Block 3 hasn't been written, and reads from memory.
        assert!(file_len() < u64::from(offsets[4]));
        assert_eq!(gf.read_game_data(offsets[4], 10).unwrap(), records[4]);
        for (record, &offset) in records.iter().zip(&offsets) {
            let read = gf.read_game_data(offset, record.len() as u32).unwrap();
            assert_eq!(&read, record, "{offset}");
        }

        // Adding after reading block 0 reloads the last block.
        assert_eq!(gf.read_game_data(0, 1).unwrap(), [1]);
        assert_eq!(gf.add_game_data(&[6; 5]).unwrap(), 3 * block + 10);
        assert_eq!(gf.read_game_data(0, 1).unwrap(), [1]);
        assert_eq!(
            gf.read_game_data(3 * block, 15).unwrap(),
            [[5; 10].as_slice(), &[6; 5]].concat()
        );
        gf.close().unwrap();

        assert_eq!(file_len(), 3 * u64::from(block) + 15);
        let mut gf = GFile::open(&path).unwrap();
        for (record, &offset) in records.iter().zip(&offsets) {
            let read = gf.read_game_data(offset, record.len() as u32).unwrap();
            assert_eq!(&read, record, "{offset}");
        }
        assert_eq!(gf.read_game_data(3 * block + 10, 5).unwrap(), [6; 5]);
    }

    #[test]
    fn test_add_game_refuses_a_record_past_the_largest_offset() {
        let temp_dir = TempDir::new().unwrap();
        let mut gf = GFile::create(temp_dir.path().join("test")).unwrap();
        let block = GF_BLOCKSIZE as u32;
        // The file seems to end 10 bytes into block 32,767, the last whose
        // offsets fit in 32 bits, without writing 4 GiB.
        let last_block = u32::MAX / block;
        gf.num_blocks = last_block + 1;
        gf.last_block_size = 10;
        gf.current_block.block_num = last_block as i32;
        gf.current_block.length = 10;

        let offset = last_block * block + 10;
        assert_eq!(gf.add_game_data(&[1; 5]).unwrap(), offset);
        assert!(matches!(
            gf.add_game_data(&vec![2; MAX_GAME_LENGTH]),
            Err(Error::GameFileFull)
        ));
        // The failed record changed nothing, so a smaller one still fits.
        assert_eq!(gf.add_game_data(&[3; 5]).unwrap(), offset + 5);
        assert_eq!(
            gf.read_game_data(offset, 10).unwrap(),
            [[1; 5], [3; 5]].concat()
        );

        // Don't write the block 4 GiB into the file.
        gf.current_block.dirty = false;
    }
}
