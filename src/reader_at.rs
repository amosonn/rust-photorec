//
// A struct for reading (impl Read) from a reader (usu. disk) according to
// a DescRead descriptor of the mapping from disk to file.
//
use fuse_fl::{ReadFileLike, Result};

use super::byte_runs::{Desc, DescRead};


pub struct ByteRunsReaderAt<R, D> {
    describer: D,
    inner: R,
}

impl<R, D> ByteRunsReaderAt<R, D> {
    pub fn new(reader: R, describer: D) -> Self {
        ByteRunsReaderAt {
            describer: describer,
            inner: reader,
        }
    }
}


impl<R, D> ReadFileLike for ByteRunsReaderAt<R, D> where R: ReadFileLike, D: for<'a> Desc<'a> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        let mut bytes_read = 0;
        let mut describer = self.describer.at_pos(offset as u64);
        loop {
            let desc = describer.desc_read();
            if desc.len == 0 { break; }
            let len = desc.len as usize;
            if (len as usize) < buf.len() - bytes_read {
                let buf2 = &mut buf[bytes_read..bytes_read+len];
                let n = self.inner.read_at(buf2, desc.disk_pos)?;
                describer.adv(n);
                bytes_read += n;
            } else {
                {
                let buf2 = &mut buf[bytes_read..];
                bytes_read += self.inner.read_at(buf2, desc.disk_pos)?;
                }
                assert_eq!(bytes_read, buf.len());
                break;
            }
        }
        Ok(bytes_read)
    }
}



#[cfg(test)]
mod tests {
    use super::super::byte_runs::{ByteRunsRef, ByteRun};
    use super::ByteRunsReaderAt;
    use fuse_fl::ReadFileLike;

    #[test]
    fn test_byte_runs_reader_at_short() {
        let br = ByteRunsRef::new(18, vec![
            ByteRun { file_offset: 0, disk_pos: 0, len: 6 },
            ByteRun { file_offset: 6, disk_pos: 10, len: 6 },
            ByteRun { file_offset: 12, disk_pos: 20, len: 6 },
        ]).unwrap();
        let reader = (0..26).collect::<Vec<u8>>();
        let reader = reader.as_slice();
        let brr = ByteRunsReaderAt {
            describer: br,
            inner: reader,
        };
        let mut out = vec![0; 4];
        assert_eq!(brr.read_at(out.as_mut_slice(), 0).unwrap(), 4);
        assert_eq!(out, vec![0, 1, 2, 3]);
        assert_eq!(brr.read_at(out.as_mut_slice(), 1).unwrap(), 4);
        assert_eq!(out, vec![1, 2, 3, 4]);
        assert_eq!(brr.read_at(out.as_mut_slice(), 2).unwrap(), 4);
        assert_eq!(out, vec![2, 3, 4, 5]);
        assert_eq!(brr.read_at(out.as_mut_slice(), 3).unwrap(), 4);
        assert_eq!(out, vec![3, 4, 5, 10]);
    }

    #[test]
    fn test_byte_runs_reader_at_long() {
        let br = ByteRunsRef::new(18, vec![
            ByteRun { file_offset: 0, disk_pos: 0, len: 6 },
            ByteRun { file_offset: 6, disk_pos: 10, len: 6 },
            ByteRun { file_offset: 12, disk_pos: 20, len: 6 },
        ]).unwrap();
        let reader = (0..26).collect::<Vec<u8>>();
        let reader = reader.as_slice();
        let brr = ByteRunsReaderAt {
            describer: br,
            inner: reader,
        };
        let mut out = vec![0; 10];
        assert_eq!(brr.read_at(out.as_mut_slice(), 4).unwrap(), 10);
        assert_eq!(out, vec![4, 5, 10, 11, 12, 13, 14, 15, 20, 21]);
    }

    #[test]
    fn test_byte_runs_reader_at_eof() {
        let br = ByteRunsRef::new(18, vec![
            ByteRun { file_offset: 0, disk_pos: 0, len: 6 },
            ByteRun { file_offset: 6, disk_pos: 10, len: 6 },
            ByteRun { file_offset: 12, disk_pos: 20, len: 6 },
        ]).unwrap();
        let reader = (0..26).collect::<Vec<u8>>();
        let reader = reader.as_slice();
        let brr = ByteRunsReaderAt {
            describer: br,
            inner: reader,
        };
        let mut out = vec![0; 5];
        assert_eq!(brr.read_at(out.as_mut_slice(), 15).unwrap(), 3);
        assert_eq!(out, vec![23, 24, 25, 0, 0]);
    }
}
