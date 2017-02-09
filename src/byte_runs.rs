
use std::io::{Seek, SeekFrom};
use std::io;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
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
    pub fn new(size: u64, mut runs: Vec<ByteRun>) -> Self {
        runs.sort();
        let gross_size = &runs.iter().map(|br| br.len).sum();
        // FIXME: check that the runs actually make one whole file!
        runs.split_last_mut().unwrap().0.len -= gross_size - size;
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

    pub fn desc_read(&mut self, n: u64) -> ByteRun {
        let mut ret = ByteRun {
            file_offset: self.pos,
            disk_pos: 0,
            len: 0,
        };
        if self.cur_run != self.runs.len() {
            ret.disk_pos = self.runs[cur_run].disk_pos + self.offset_in_run;
            let rem = self.runs[cur_run].len - self.offset_in_run;
            if n < rem {
                ret.len = n;
                self.pos += n;
                self.offset_in_run += n;
            } else {
                ret.len = rem;
                self.pos += rem;
                self.cur_run += 1;
                self.offset_in_run = 0;
            }
        }
        ret
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

#[test]
fn test_byte_runs_ref_ctor() {
    let br = ByteRunsRef::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]);
    assert_eq!(br.size, 123);
    assert_eq!(br.runs[0], ByteRun { file_offset: 0, disk_pos: 16000, len: 50});
    assert_eq!(br.runs[1], ByteRun { file_offset: 50, disk_pos: 8000, len: 50});
    assert_eq!(br.runs[2], ByteRun { file_offset: 100, disk_pos: 2000, len: 23});
}

#[test]
fn test_byte_runs_ref_seek() {
    let mut br = ByteRunsRef::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]);
    assert_eq!(br.seek(SeekFrom::Start(3)).unwrap(), 3);
    assert_eq!(br.seek(SeekFrom::Start(6)).unwrap(), 6);
    assert_eq!(br.seek(SeekFrom::Current(0x7ffffffffffffff0)).unwrap(), 0x7ffffffffffffff6);
    assert_eq!(br.seek(SeekFrom::Current(0x10)).unwrap(), 0x8000000000000006);
    assert!(br.seek(SeekFrom::Current(0x7ffffffffffffffd)).is_err());
    assert_eq!(br.seek(SeekFrom::Current(-0x8000000000000000)).unwrap(), 6);
    assert_eq!(br.seek(SeekFrom::End(10)).unwrap(), 133);
    assert_eq!(br.seek(SeekFrom::End(-10)).unwrap(), 113);
    assert!(br.seek(SeekFrom::End(-1000)).is_err());
}
