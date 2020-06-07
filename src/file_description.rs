//
// The description of a "fileobject" - a collection of ByteRuns, mapping from
// a byte run in the disk to the parts of a file.
//
use std::io::{Seek, SeekFrom};
use std::io;
use std::fmt;
use std::mem;

use thiserror::Error;

use crate::segment_tree::Segment;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct ByteRun {
    pub file_offset: u64,
    pub disk_pos: u64,
    pub len: u64,
}

impl fmt::Display for ByteRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(file_offset: {}, disk_pos: {}, len: {})", self.file_offset, self.disk_pos, self.len)
    }
}

impl From<&ByteRun> for Segment {
    fn from(br: &ByteRun) -> Self {
        Segment { start: br.disk_pos, end: br.disk_pos + br.len }
    }
}

pub trait DescRead {
    fn desc_read(&mut self) -> ByteRun;
    fn adv(&mut self, n: usize);
}

// FIXME: will replace once Associated Type Constructors (PR RFC #1598) lands.
pub trait Desc<'a> {
    type DescReader: DescRead;
    fn at_pos(&'a self, pos: u64) -> Self::DescReader;
}

#[derive(Debug, Clone)]
pub struct FileDescription {
    runs: Box<[ByteRun]>,
    size: u64,
}

#[derive(Debug)]
pub struct FileDescriptionPos<'a> {
    _ref: &'a FileDescription,
    pos: u64,
    cur_run: usize,
    offset_in_run: u64,
}

#[derive(Error, Debug)]
pub enum FileDescriptionError {
    #[error("Error constructing FileDescription: {0} and {1} are overlapping")]
    Overlap(ByteRun, ByteRun),
    #[error("Error constructing FileDescription: Gap between {0} and {1}")]
    Gap(ByteRun, ByteRun),
    #[error("Error constructing FileDescription: Gap between beginning and {0}")]
    PreGap(ByteRun),
    #[error("Error constructing FileDescription: No ByteRuns given")]
    Empty,
    #[error("Error constructing FileDescription: {0} is already past given size {1}")]
    Trailing(ByteRun, u64),
    #[error("Error constructing FileDescription: Should be of size {0}, but only size {1} is covered")]
    Missing(u64, u64)
}

impl FileDescription {
    pub fn new<T: IntoIterator<Item=ByteRun>>(size: u64, runs: T) -> Result<Self, FileDescriptionError> {
        let mut runs: Vec<ByteRun> = runs.into_iter().collect();
        if runs.len() == 0 { return Err(FileDescriptionError::Empty); }
        runs.sort_unstable();
        let runs = runs;

        let mut off = 0;
        let mut it = runs.iter();
        let mut br = it.next().unwrap();
        // First check the first ByteRun starts at 0
        if br.file_offset != 0 { return Err(FileDescriptionError::PreGap(*br)); }
        off += br.len;
        // Then check all the following ByteRun-s come one after another
        for br2 in it {
            if off > size { return Err(FileDescriptionError::Trailing(*br2, size)); }
            if br2.file_offset > off { return Err(FileDescriptionError::Gap(*br, *br2)); }
            else if br2.file_offset < off { return Err(FileDescriptionError::Overlap(*br, *br2)); }
            br = br2;
            off += br.len;
        }

        if size > off { return Err(FileDescriptionError::Missing(size, off)); }
        // The last block sometimes needs to be trimmed
        let mut runs = runs;
        runs.last_mut().unwrap().len -= off - size;
        Ok(FileDescription {
            runs: runs.into_boxed_slice(),
            size: size,
        })
    }

    pub fn size(&self) -> u64 { self.size }
}

impl AsRef<[ByteRun]> for FileDescription {
    fn as_ref(&self) -> &[ByteRun] { &self.runs }
}

impl<'a> Desc<'a> for FileDescription {
    type DescReader = FileDescriptionPos<'a>;

    fn at_pos(&'a self, pos: u64) -> FileDescriptionPos<'a> {
        if pos > self.size {
            FileDescriptionPos {
                _ref: &self,
                pos,
                cur_run: self.runs.len(),
                offset_in_run: 0,
            }
        } else {
            let cur_run = match self.runs.binary_search_by_key(&pos, |br| br.file_offset) {
                    Ok(x) => x,  // We're at the beginning of this slice.
                    Err(x) => x-1,  // We could be inserted after this slice, which
                                    // means we're somewhere within it.
                };
            FileDescriptionPos {
                _ref: &self,
                pos,
                cur_run,
                offset_in_run: pos - self.runs[cur_run].file_offset,
            }
        }
    }
}

impl<'a> From<&'a FileDescription> for FileDescriptionPos<'a> {
    fn from(_ref: &FileDescription) -> FileDescriptionPos<'_> {
        _ref.at_pos(0)
    }
}

impl<'a> DescRead for FileDescriptionPos<'a> {
    fn desc_read(&mut self) -> ByteRun {
        if self.cur_run != self._ref.runs.len() {
            ByteRun {
                file_offset: self.pos,
                disk_pos: self._ref.runs[self.cur_run].disk_pos + self.offset_in_run,
                len: self._ref.runs[self.cur_run].len - self.offset_in_run,
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
        let rem = self._ref.runs[self.cur_run].len - self.offset_in_run;
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

impl<'a> Seek for FileDescriptionPos<'a> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match pos {
            SeekFrom::Start(x) => { mem::swap(self, &mut self._ref.at_pos(x)); return Ok(x); }
            SeekFrom::Current(x) => (self.pos, x),
            SeekFrom::End(x) => (self._ref.size, x),
        };
        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };
        match new_pos {
            Some(x) => { mem::swap(self, &mut self._ref.at_pos(x)); return Ok(x) },
            None => Err(io::Error::new(io::ErrorKind::InvalidInput, "Bad seek pos.")),
        }
    }
}

#[test]
fn test_file_description_ref_ctor() {
    let br = FileDescription::new(123, vec![
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
fn test_file_description_ref_ctor_integrity() {
    if let Err(FileDescriptionError::Empty) = FileDescription::new(123, vec![
    ]) {
    } else { panic!(); }
    if let Err(FileDescriptionError::PreGap(y)) = FileDescription::new(123, vec![
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
    ]) {
        assert_eq!(y, ByteRun { file_offset: 50, disk_pos: 8000, len: 50});
    } else { panic!(); }
    if let Err(FileDescriptionError::Gap(x, y)) = FileDescription::new(123, vec![
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]) {
        assert_eq!(x, ByteRun { file_offset: 0, disk_pos: 16000, len: 50});
        assert_eq!(y, ByteRun { file_offset: 100, disk_pos: 2000, len: 50});
    } else { panic!(); }
    if let Err(FileDescriptionError::Overlap(x, y)) = FileDescription::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 60 },
    ]) {
        assert_eq!(x, ByteRun { file_offset: 0, disk_pos: 16000, len: 60});
        assert_eq!(y, ByteRun { file_offset: 50, disk_pos: 8000, len: 50});
    } else { panic!(); }
}

#[test]
fn test_file_description_ref_pos_seek() {
    let br = FileDescription::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]).unwrap();
    let mut brf = FileDescriptionPos::from(&br);
    assert_eq!(brf.seek(SeekFrom::Start(3)).unwrap(), 3);
    assert_eq!(brf.seek(SeekFrom::Start(6)).unwrap(), 6);
    assert_eq!(brf.seek(SeekFrom::Current(0x7ffffffffffffff0)).unwrap(), 0x7ffffffffffffff6);
    assert_eq!(brf.seek(SeekFrom::Current(0x10)).unwrap(), 0x8000000000000006);
    assert!(brf.seek(SeekFrom::Current(0x7ffffffffffffffd)).is_err());
    assert_eq!(brf.seek(SeekFrom::Current(-0x8000000000000000)).unwrap(), 6);
    assert_eq!(brf.seek(SeekFrom::End(10)).unwrap(), 133);
    assert_eq!(brf.seek(SeekFrom::End(-10)).unwrap(), 113);
    assert!(brf.seek(SeekFrom::End(-1000)).is_err());
}

#[test]
fn test_file_description_ref_at_pos() {
    let br = FileDescription::new(123, vec![
        ByteRun { file_offset: 50, disk_pos: 8000, len: 50 },
        ByteRun { file_offset: 100, disk_pos: 2000, len: 50 },
        ByteRun { file_offset: 0, disk_pos: 16000, len: 50 },
    ]).unwrap();
    let brp = br.at_pos(0);
    assert_eq!(brp.pos, 0);
    assert_eq!(brp.cur_run, 0);
    assert_eq!(brp.offset_in_run, 0);
    let brp = br.at_pos(70);
    assert_eq!(brp.pos, 70);
    assert_eq!(brp.cur_run, 1);
    assert_eq!(brp.offset_in_run, 20);
    let brp = br.at_pos(170);
    assert_eq!(brp.pos, 170);
    assert_eq!(brp.cur_run, 3);
    assert_eq!(brp.offset_in_run, 0);
}
