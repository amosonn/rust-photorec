
use std::ffi::OsString;
use std::path::Path;
use std::fs::File;
use std::path::{PathBuf, Component};
use std::collections::HashMap;
use fuse_fl::{FilesystemFL, ResultOpenObj, RequestInfo};
use fuse_fl::filelike::{NoFile, FilesystemFLRwOpen};
use libc;

use super::byte_runs::ByteRunsRef;
use super::reader_at::ByteRunsReaderAt;

#[derive(Serialize, Deserialize)]
enum NodeType {
    Brf(ByteRunsRef),
    Dir,
}

#[derive(Serialize, Deserialize)]
pub struct PhotorecFS {
    vfs: HashMap<PathBuf, NodeType>,
    disk_path: OsString,
}

impl PhotorecFS {
    fn new<T: IntoIterator<Item = (OsString, ByteRunsRef)>>(brfs: T, disk_path: OsString) -> PhotorecFS {
        let mut vfs = HashMap::new();
        for (name, content) in brfs {
            let mut path = PathBuf::new();
            for part in PathBuf::from(name).components() {
                if let Component::Normal(part) = part {
                    path.push(part);
                    if vfs.insert(path.clone(), NodeType::Dir).is_some() {
                        panic!() // TODO: put normal error.
                    }
                } else {
                    panic!() // TODO: put normal error.
                }
            }
            vfs.insert(path.clone(), NodeType::Brf(content));
        }
        PhotorecFS { vfs, disk_path }
    }
}

impl FilesystemFLRwOpen for PhotorecFS {
    type ReadLike = ByteRunsReaderAt<File, ByteRunsRef>;
    type WriteLike = NoFile;
    type ReadWriteLike = NoFile;

    fn open_read(&self,
                 _req: RequestInfo,
                 _path: &Path,
                 _flags: u32)
                 -> ResultOpenObj<Self::ReadLike> {
        let f = File::open(&self.disk_path).unwrap();
        match self.vfs.get(_path) {
            Some(&NodeType::Brf(ref x)) => Ok((ByteRunsReaderAt::new(f, x.clone()), 0)),
            Some(&NodeType::Dir) => Err(libc::EEXIST),
            None => Err(libc::ENOENT),
        }
    }
}

