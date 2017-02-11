
use std::io::{Read, Seek, SeekFrom};
use std::io;
use std::fs::File;

use super::byte_runs::ByteRunsRef;


pub struct ByteRunsReader {
    brf: ByteRunsRef,
    file: File,
}

//impl ByteRunsReader {
    //pub fn new() -> Self {
        //ByteRunsReader {

    //}


impl Seek for ByteRunsReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> { self.brf.seek(pos) }
}


impl Read for ByteRunsReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let desc = self.brf.desc_read(buf.len());
        if desc.len == 0 { return Ok(0); }
        // FIXME types.
        let buf2 = &mut buf[..(desc.len as usize)];
        // FIXME if read is too short, brf is out of sync.
        self.file.seek(SeekFrom::Start(desc.disk_pos)).and_then(|_| self.file.read(buf2))
    }
}
