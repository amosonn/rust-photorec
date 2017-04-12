
use std::ffi::OsString;
use std::path::Path;
use std::fs::File;
use std::path::{PathBuf, Component};
use std::collections::HashMap;
use fuse_fl::{FilesystemFL, ResultOpenObj, RequestInfo, DirectoryEntry, FileType};
use fuse_fl::filelike::{NoFile, FilesystemFLRwOpen, FilesystemFLOpen, ModalFileLike};
use libc;

use super::byte_runs::ByteRunsRef;
use super::reader_at::ByteRunsReaderAt;

#[derive(Serialize, Deserialize)]
enum MyFileType {
    RegularFile,
    Directory,
}

impl From<MyFileType> for FileType {
    fn from(mft: MyFileType) -> FileType {
        match mft {
            MyFileType::RegularFile => FileType::RegularFile,
            MyFileType::Directory => FileType::Directory,
        }
    }
}

#[derive(Serialize, Deserialize)]
enum NodeType {
    Brf(ByteRunsRef),
    Dir(HashMap<OsString, MyFileType>),
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
            let _temp = PathBuf::from(name);
            let mut iter = _temp.components();
            let fname = match iter.next_back() {
                None => panic!(), // TODO put normal error.
                Some(Component::Normal(ref fname)) => fname.clone(), // This only clones the ref!
                Some(_) => panic!(), // TODO put normal error.
            };
            let mut path = match iter.next() {
                None => PathBuf::new(),
                Some(Component::Normal(ref first)) => PathBuf::from(first),
                Some(_) => panic!(), // TODO put normal error.
            };
            for part in iter {
                if let Component::Normal(part) = part {
                    match vfs.entry(path.clone()).or_insert_with(|| NodeType::Dir(HashMap::new())) {
                        &mut NodeType::Brf(_) => panic!(), // TODO put normal error.
                        &mut NodeType::Dir(ref mut dir_contents) => {
                            match dir_contents.insert(part.to_os_string(), MyFileType::Directory) {
                                Some(MyFileType::Directory) => {}
                                Some(_) => panic!(), // TODO put normal error.
                                None => {}
                            }
                        }
                    }
                    path.push(part);
                } else {
                    panic!() // TODO: put normal error.
                }
            }
            match vfs.entry(path.clone()).or_insert_with(|| NodeType::Dir(HashMap::new())) {
                &mut NodeType::Brf(_) => panic!(), // TODO put normal error.
                &mut NodeType::Dir(ref mut dir_contents) => {
                    match dir_contents.insert(fname.to_os_string(), MyFileType::RegularFile) {
                        Some(MyFileType::RegularFile) => {}
                        Some(_) => panic!(), // TODO put normal error.
                        None => {}
                    }
                }
            }
            path.push(fname);
            assert!(vfs.insert(path, NodeType::Brf(content)).is_none());
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
            Some(&NodeType::Dir(_)) => Err(libc::EEXIST),
            None => Err(libc::ENOENT),
        }
    }
}

