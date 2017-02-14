
use std::io::{Read, Seek, SeekFrom};
use std::io;

use super::byte_runs::{DescRead, ByteRunsRef};


pub struct ByteRunsReader<R, D = ByteRunsRef> {
    describer: D,
    inner: R,
}

//impl ByteRunsReader {
    //pub fn new() -> Self {
        //ByteRunsReader {

    //}


impl<R, D: Seek> Seek for ByteRunsReader<R, D> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> { self.describer.seek(pos) }
}


impl<R: Read+Seek, D: DescRead> Read for ByteRunsReader<R, D> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let desc = self.describer.desc_read(buf.len());
        if desc.len == 0 { return Ok(0); }
        // FIXME types.
        let buf2 = &mut buf[..(desc.len as usize)];
        // FIXME if read is too short, brf is out of sync.
        self.inner.seek(SeekFrom::Start(desc.disk_pos)).and_then(|_| self.inner.read(buf2))
    }
}
