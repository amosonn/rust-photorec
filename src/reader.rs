
use std::io::{Read, Seek, SeekFrom};
use std::io;

use super::byte_runs::{DescRead, ByteRunsRef};


pub struct ByteRunsReader<R, D = ByteRunsRef> {
    describer: D,
    inner: R,
}

impl<R, D> ByteRunsReader<R, D> {
    pub fn new(reader: R, describer: D) -> Self {
        ByteRunsReader {
            describer: describer,
            inner: reader,
        }
    }
}


impl<R, D: Seek> Seek for ByteRunsReader<R, D> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> { self.describer.seek(pos) }
}


impl<R: Read+Seek, D: DescRead> Read for ByteRunsReader<R, D> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let desc = self.describer.desc_read();
        if desc.len == 0 { return Ok(0); }
        let buf2 = &mut buf[..(desc.len as usize)];
        self.inner.seek(SeekFrom::Start(desc.disk_pos))
            .and_then(|_| self.inner.read(buf2))
            .and_then(|n| {self.describer.adv(n); Ok(n)})
    }
}


#[test]
fn test_byte_runs_reader_easy() {
    use super::byte_runs::ByteRun;
    let br = ByteRunsRef::new(18, vec![
        ByteRun { file_offset: 0, disk_pos: 0, len: 6 },
        ByteRun { file_offset: 6, disk_pos: 10, len: 6 },
        ByteRun { file_offset: 12, disk_pos: 20, len: 6 },
    ]).unwrap();
    let reader = io::Cursor::new((0..26).collect::<Vec<u8>>());
    let mut brr = ByteRunsReader {
        describer: br,
        inner: reader,
    };
    let mut out = Vec::<u8>::with_capacity(18);
    assert_eq!(brr.read_to_end(&mut out).unwrap(), 18);
    assert_eq!(out, vec![0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25]);
}


#[test]
fn test_byte_runs_reader_hard() {
    use super::byte_runs::ByteRun;

    struct LameCursor<T> {
        inner: io::Cursor<T>,
    };

    impl<T> LameCursor<T> {
        fn new(t: T) -> Self { LameCursor { inner: io::Cursor::new(t) } }
    }

    impl<T: AsRef<[u8]>> Seek for LameCursor<T> {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> { self.inner.seek(pos) }
    }

    impl<T: AsRef<[u8]>> Read for LameCursor<T> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if buf.len() <= 3 {
                self.inner.read(buf)
            } else {
                self.inner.read(&mut buf[..3])
            }
        }
    }

    let br = ByteRunsRef::new(18, vec![
        ByteRun { file_offset: 0, disk_pos: 0, len: 6 },
        ByteRun { file_offset: 6, disk_pos: 10, len: 6 },
        ByteRun { file_offset: 12, disk_pos: 20, len: 6 },
    ]).unwrap();
    let reader = LameCursor::new((0..26).collect::<Vec<u8>>());
    let mut brr = ByteRunsReader {
        describer: br,
        inner: reader,
    };
    let mut out = Vec::<u8>::with_capacity(18);
    assert_eq!(brr.read_to_end(&mut out).unwrap(), 18);
    assert_eq!(out, vec![0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25]);
}
