use std::io::{ErrorKind, Read};

use super::{Location, Raw, ReadErrorKind, Source};

/// Fixed-size records with no separators.
#[derive(Debug)]
pub struct BlockSource<R> {
    reader: R,
    record_size: usize,
    /// Byte offset of the most recently attempted block.
    offset: u64,
    /// Byte offset of the next block.
    next_offset: u64,
}

impl<R: Read> BlockSource<R> {
    /// Reads blocks of `record_size` bytes from `reader`.
    ///
    /// # Panics
    ///
    /// Panics if `record_size` is zero.
    pub fn new(reader: R, record_size: usize) -> Self {
        assert!(record_size > 0, "record_size must be at least 1");
        Self {
            reader,
            record_size,
            offset: 0,
            next_offset: 0,
        }
    }

    /// Returns the wrapped reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<R: Read> Source for BlockSource<R> {
    fn next_raw(&mut self) -> Result<Option<Raw>, ReadErrorKind> {
        let mut bytes = vec![0; self.record_size];
        let mut filled = 0;
        while filled < bytes.len() {
            match self.reader.read(&mut bytes[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => return Err(err.into()),
            }
        }
        if filled == 0 {
            return Ok(None);
        }
        self.offset = self.next_offset;
        self.next_offset += filled as u64;
        if filled < self.record_size {
            return Err(ReadErrorKind::Truncated {
                expected: self.record_size,
                actual: filled,
            });
        }
        Ok(Some(Raw {
            bytes,
            terminator: Vec::new(),
            location: Location::Offset(self.offset),
        }))
    }

    fn location(&self) -> Location {
        Location::Offset(self.offset)
    }
}
