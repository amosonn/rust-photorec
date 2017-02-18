//
// The description of a "fileobject" - a collection of ByteRuns, mapping from
// a byte run in the disk to the parts of a file.
//
use std::io::{Seek, SeekFrom};
use std::io;
use std::fmt;
use std::error::Error;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct ByteRun {
    pub file_offset: u64,
    pub disk_pos: u64,
    pub len: u64,
}

impl fmt::Display for ByteRun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(file_offset: {}, disk_pos: {}, len: {})", self.file_offset, self.disk_pos, self.len)
    }
}

pub trait DescRead {
    fn desc_read(&mut self) -> ByteRun;
    fn adv(&mut self, n: usize);
}

#[derive(Debug)]
pub struct ByteRunsRef {
    runs: Box<[ByteRun]>,
    size: u64,
    pos: u64,
    cur_run: usize,
    offset_in_run: u64,
}

#[derive(Debug)]
pub enum ByteRunsRefError {
    Overlap(ByteRun, ByteRun),
    Gap(ByteRun, ByteRun),
    PreGap(ByteRun),
    Empty,
}

impl fmt::Display for ByteRunsRefError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ByteRunsRefError::Overlap(x, y) => write!(f, "Error constructing ByteRunsRef: {} and {} are overlapping", x, y),
            ByteRunsRefError::Gap(x, y) => write!(f, "Error constructing ByteRunsRef: Gap between {} and {}", x, y),
            ByteRunsRefError::PreGap(y) => write!(f, "Error constructing ByteRunsRef: Gap between beginning and {}", y),
            ByteRunsRefError::Empty => write!(f, "Error constructing ByteRunsRef: No ByteRuns given"),
        }
    }
}

impl Error for ByteRunsRefError {
    fn description(&self) -> &str {
        match *self {
            ByteRunsRefError::Overlap(_, _) => "Overlapping ByteRuns.",
            ByteRunsRefError::Gap(_, _) => "Gap between ByteRuns.",
            ByteRunsRefError::PreGap(_) => "Gap before ByteRuns.",
            ByteRunsRefError::Empty => "No ByteRuns.",
        }
    }
}

impl ByteRunsRef {
    pub fn new(size: u64, mut runs: Vec<ByteRun>) -> Result<Self, ByteRunsRefError> {
        if runs.len() == 0 { return Err(ByteRunsRefError::Empty); }
        runs.sort();
        let mut off = 0;
        {
            let mut it = runs.iter();
            let mut br = it.next().unwrap();
            if br.file_offset != 0 { return Err(ByteRunsRefError::PreGap(*br)); }
            off += br.len;
            for br2 in it {
                if br2.file_offset > off { return Err(ByteRunsRefError::Gap(*br, *br2)); }
                else if br2.file_offset < off { return Err(ByteRunsRefError::Overlap(*br, *br2)); }
                br = br2;
                off += br.len;
            }
        }
        // We could do this inside, but then the entire iter has to be mut...
        runs.last_mut().unwrap().len -= off - size;
        Ok(ByteRunsRef {
            runs: runs.into_boxed_slice(),
            size: size,
            pos: 0,
            cur_run: 0,
            offset_in_run: 0,
        })
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

impl DescRead for ByteRunsRef {
    fn desc_read(&mut self) -> ByteRun {
        if self.cur_run != self.runs.len() {
            ByteRun {
                file_offset: self.pos,
                disk_pos: self.runs[self.cur_run].disk_pos + self.offset_in_run,
                len: self.runs[self.cur_run].len - self.offset_in_run,
            }
        } else {
            ByteRun {
                file_offset: self.pos,
                disk_pos: 0,
                len: 0,
            }
        }
    }

    fn adv(&mut self, n: usize) {
        let n = n as u64;
        let rem = self.runs[self.cur_run].len - self.offset_in_run;
        if n < rem {
            self.pos += n;
            self.offset_in_run += n;
        } else if n == rem {
            self.pos += rem;
            self.cur_run += 1;
            self.offset_in_run = 0;
        } else {
            panic!("Should only read up to end of ByteRun.")
        }
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
    ]).unwrap();
    assert_eq!(br.size, 123);
    assert_eq!(br.runs[0], ByteRun { file_offset: 0, disk_pos: 16000, len: 50});
    assert_eq!(br.runs[1], ByteRun { file_offset: 50, disk_pos: 8000, len: 50});
    assert_eq!(br.runs[2], ByteRun { file_offset: 100, disk_pos: 2000, len: 23});
}

#[test]
fn test_byte_runs_ref_ctor_integrity() {
    if let Err(ByteRunsRefError::Empty) = ByteRunsRef::new(123, vec![
    ]) {
    } else { panic!(); }
    if let Err(ByteRunsRefError::PreGap(y)) = ByteRunsRef::new(123, vec![
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
    ]) {
        assert_eq!(y, ByteRun { file_offset: 50, disk_pos: 8000, len: 50});
    } else { panic!(); }
    if let Err(ByteRunsRefError::Gap(x, y)) = ByteRunsRef::new(123, vec![
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]) {
        assert_eq!(x, ByteRun { file_offset: 0, disk_pos: 16000, len: 50});
        assert_eq!(y, ByteRun { file_offset: 100, disk_pos: 2000, len: 50});
    } else { panic!(); }
    if let Err(ByteRunsRefError::Overlap(x, y)) = ByteRunsRef::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 60 },
    ]) {
        assert_eq!(x, ByteRun { file_offset: 0, disk_pos: 16000, len: 60});
        assert_eq!(y, ByteRun { file_offset: 50, disk_pos: 8000, len: 50});
    } else { panic!(); }
}

#[test]
fn test_byte_runs_ref_seek() {
    let mut br = ByteRunsRef::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]).unwrap();
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
