//! The text `PgnParser` reads, decoded from its reader a block at a time.

use std::io::{self, Read};
use std::sync::{Mutex, PoisonError};

use crate::pgn::windows_1252_char;

/// How many bytes each read asks for.
const READ_SIZE: usize = 64 * 1024;

/// A window of the decoded input. Bytes that aren't valid UTF-8 are read as
/// Windows-1252, as if the whole input were decoded at once: a character
/// split between two reads is joined, and only a sequence that is invalid
/// whatever follows it is read as Windows-1252.
///
/// [`fill`](Self::fill) adds to the window, and [`drop_before`](Self::drop_before)
/// drops text the parser no longer needs. Positions are indexes into the
/// window, so dropping text moves them.
pub(crate) struct PgnInput<'a> {
    /// The reader, in a `Mutex` only so that the parser is `Sync` with a
    /// reader that is just `Send`. It is reached through `get_mut`, which
    /// doesn't lock.
    reader: Mutex<Box<dyn Read + Send + 'a>>,
    /// The decoded text.
    chars: Vec<char>,
    /// Where bytes that weren't valid UTF-8 were read as Windows-1252: the
    /// index in `chars` and the line of each run of them, in input order.
    not_utf8: Vec<(usize, u32)>,
    /// The buffer each read fills.
    buffer: Vec<u8>,
    /// Bytes read but not yet decoded: the start of a character that the
    /// next read may complete.
    bytes: Vec<u8>,
    /// The line the next decoded character is on.
    line: u32,
    /// Whether the reader is done, at its end or after an error.
    done: bool,
    /// The error that stopped reading, until it is taken.
    error: Option<io::Error>,
}

impl<'a> PgnInput<'a> {
    pub(crate) fn new(reader: impl Read + Send + 'a) -> Self {
        PgnInput {
            reader: Mutex::new(Box::new(reader)),
            chars: Vec::new(),
            not_utf8: Vec::new(),
            buffer: vec![0; READ_SIZE],
            bytes: Vec::new(),
            line: 1,
            done: false,
            error: None,
        }
    }

    /// The decoded text in the window.
    pub(crate) fn chars(&self) -> &[char] {
        &self.chars
    }

    /// The runs of bytes in the window that weren't valid UTF-8, as their
    /// index in [`chars`](Self::chars) and their line.
    pub(crate) fn not_utf8(&self) -> &[(usize, u32)] {
        &self.not_utf8
    }

    /// Decodes more of the input into the window. Returns `false` if there
    /// is no more, because the reader is at its end or failed.
    pub(crate) fn fill(&mut self) -> bool {
        let before = self.chars.len();
        while self.chars.len() == before && !self.done {
            // Nothing locks the mutex, so it can't be poisoned.
            let reader = self
                .reader
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner);
            let read = loop {
                match reader.read(&mut self.buffer) {
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                    result => break result,
                }
            };
            let read = read.unwrap_or_else(|err| {
                self.error = Some(err);
                0
            });
            self.bytes.extend_from_slice(&self.buffer[..read]);
            self.done = read == 0;
            self.decode();
        }
        self.chars.len() > before
    }

    /// Takes the error that stopped reading, if any.
    pub(crate) fn take_error(&mut self) -> Option<io::Error> {
        self.error.take()
    }

    /// Drops the text before `keep`, if that is at least half of the window,
    /// so that dropping takes time in proportion to the text dropped.
    /// Returns how many characters were dropped, by which every position in
    /// the window moves back.
    pub(crate) fn drop_before(&mut self, keep: usize) -> usize {
        if keep == 0 || keep * 2 < self.chars.len() {
            return 0;
        }
        self.chars.drain(..keep);
        let dropped = self.not_utf8.partition_point(|&(pos, _)| pos < keep);
        self.not_utf8.drain(..dropped);
        for (pos, _) in &mut self.not_utf8 {
            *pos -= keep;
        }
        keep
    }

    /// Decodes `bytes`, except for an incomplete character at their end
    /// while more may follow.
    fn decode(&mut self) {
        let mut decoded = self.bytes.len();
        let mut chunks = self.bytes.utf8_chunks().peekable();
        while let Some(chunk) = chunks.next() {
            for c in chunk.valid().chars() {
                if c == '\n' {
                    self.line += 1;
                }
                self.chars.push(c);
            }
            let invalid = chunk.invalid();
            if invalid.is_empty() {
                continue;
            }
            // The last chunk's invalid bytes end the bytes read. They may be
            // the start of a character the next read completes.
            let incomplete = std::str::from_utf8(invalid).is_err_and(|e| e.error_len().is_none());
            if chunks.peek().is_none() && !self.done && incomplete {
                decoded -= invalid.len();
                break;
            }
            // A line break is valid UTF-8, so an invalid run is on one line.
            self.not_utf8.push((self.chars.len(), self.line));
            self.chars
                .extend(invalid.iter().map(|&b| windows_1252_char(b)));
        }
        self.bytes.drain(..decoded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that returns at most `size` bytes at a time.
    struct Chunked<'a> {
        bytes: &'a [u8],
        size: usize,
    }

    impl Read for Chunked<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.size.min(buf.len()).min(self.bytes.len());
            buf[..n].copy_from_slice(&self.bytes[..n]);
            self.bytes = &self.bytes[n..];
            Ok(n)
        }
    }

    /// Decodes all of `bytes`, read `size` bytes at a time.
    fn decode_all(bytes: &[u8], size: usize) -> (String, Vec<(usize, u32)>) {
        let mut input = PgnInput::new(Chunked { bytes, size });
        while input.fill() {}
        assert!(input.take_error().is_none());
        (input.chars().iter().collect(), input.not_utf8().to_vec())
    }

    #[test]
    fn test_reads_split_characters_as_whole_input() {
        // A 2-, 3-, and 4-byte character, invalid bytes, a truncated
        // character before valid text, and one at the very end.
        let bytes = b"a\xC3\xA9b\xE2\x82\xACc\xF0\x9F\x98\x80\nd\xFF\xFEe\xE2\x82(\n\xF0\x9F\x98";
        let whole = decode_all(bytes, READ_SIZE);
        assert_eq!(
            whole.0,
            "a\u{e9}b\u{20ac}c\u{1f600}\nd\u{ff}\u{fe}e\u{e2}\u{201a}(\n\u{f0}\u{178}\u{2DC}"
        );
        assert_eq!(whole.1, [(8, 2), (9, 2), (11, 2), (15, 3)]);
        for size in 1..=bytes.len() {
            assert_eq!(decode_all(bytes, size), whole, "reads of {size} bytes");
        }
    }

    #[test]
    fn test_drop_before_moves_positions() {
        let mut input = PgnInput::new(&b"ab\xFFcd\xFE"[..]);
        while input.fill() {}
        assert_eq!(input.not_utf8(), [(2, 1), (5, 1)]);
        // Less than half of the window is kept as it is.
        assert_eq!(input.drop_before(2), 0);
        assert_eq!(input.drop_before(3), 3);
        assert_eq!(input.chars(), ['c', 'd', '\u{fe}']);
        assert_eq!(input.not_utf8(), [(2, 1)]);
    }

    #[test]
    fn test_read_error_ends_input() {
        struct Failing(bool);
        impl Read for Failing {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if std::mem::replace(&mut self.0, true) {
                    Err(io::Error::other("disk on fire"))
                } else {
                    buf[0] = b'a';
                    Ok(1)
                }
            }
        }
        let mut input = PgnInput::new(Failing(false));
        assert!(input.fill());
        assert!(!input.fill());
        assert!(!input.fill());
        assert_eq!(input.take_error().unwrap().to_string(), "disk on fire");
        assert!(input.take_error().is_none());
        assert_eq!(input.chars(), ['a']);
    }

    #[test]
    fn test_retries_interrupted_reads() {
        struct Interrupting(u8);
        impl Read for Interrupting {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.0 += 1;
                match self.0 {
                    1 => Err(io::ErrorKind::Interrupted.into()),
                    2 => {
                        buf[0] = b'a';
                        Ok(1)
                    }
                    _ => Ok(0),
                }
            }
        }
        let mut input = PgnInput::new(Interrupting(0));
        while input.fill() {}
        assert!(input.take_error().is_none());
        assert_eq!(input.chars(), ['a']);
    }
}
