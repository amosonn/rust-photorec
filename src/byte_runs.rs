
use std::io::{Seek, SeekFrom};
use std::io;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRun {
    file_offset: u64,
    disk_pos: u64,
    len: u64,
}

pub struct ByteRunsRef {
    runs: Box<[ByteRun]>,
    size: u64,
    pos: u64,
    cur_run: usize,
    offset_in_run: u64,
}

impl ByteRunsRef {
    pub fn new<T>(size: u64, it: T) -> Self where T: IntoIterator<Item=ByteRun> {
        let mut runs: Vec<ByteRun> = it.into_iter().collect();
        let gross_size = &runs.iter().map(|br| br.len).sum();
        runs.split_last_mut().unwrap().0.len -= gross_size - size;
        runs.sort();
        ByteRunsRef {
            runs: runs.into_boxed_slice(),
            size: size,
            pos: 0,
            cur_run: 0,
            offset_in_run: 0,
        }
    }

    fn set_pos(&mut self, pos: u64) -> io::Result<u64> {
        self.pos = pos;
        if pos > self.size {
            self.cur_run = self.runs.len();
            self.offset_in_run = 0;
        } else {
            self.cur_run = match self.runs.binary_search_by_key(&pos, |br| br.file_offset) {
                Ok(x) => x,  // We're at the beginning of this slice.
                Err(x) => x-1,  // We could be inserted after this slice, which
                                // means we're somewhere within it.
            };
            self.offset_in_run = self.pos - self.runs[self.cur_run].file_offset;
        }
        Ok(pos)
    }
}

impl Seek for ByteRunsRef {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match pos {
            SeekFrom::Start(x) => { return self.set_pos(x); }
            SeekFrom::Current(x) => (self.pos, x),
            SeekFrom::End(x) => (self.size, x),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };
        match new_pos {
            Some(x) => self.set_pos(x),
            None => Err(io::Error::new(io::ErrorKind::InvalidInput, "Bad seek pos.")),
        }
    }
}
