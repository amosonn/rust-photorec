
use std::ffi::{OsString, OsStr};
use std::path::Path;
use std::fs::File;
use std::path::{PathBuf, Component};
use std::collections::HashMap;
// use fuse_fl::{FilesystemFL, ResultOpenObj, ResultEmpty, RequestInfo, DirectoryEntry, FileType};
use fuse_fl::*;
use fuse_fl::filelike::{NoFile, FilesystemFLRwOpen, FilesystemFLOpen, ModalFileLike};
use libc;
use time::Timespec;

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

    fn fsync_metadata(&self,
                      _req: RequestInfo,
                      _path: &Path,
                      _fl: &ModalFileLike<Self::ReadLike, Self::WriteLike, Self::ReadWriteLike>)
                      -> ResultEmpty {
        Ok(())
        // Err(libc::ENOSYS)
    }
}

impl FilesystemFL for PhotorecFS {
    /// The type for objects returned by open/create and used by read, etc.
    type FileLike = <PhotorecFS as FilesystemFLOpen>::FileLike;
    /// The type for objects returned by opendir and used by readdir, etc.
    type DirLike = Option<u32>;

    /// Called on mount, before any other function.
    fn init(&self, _req: RequestInfo) -> ResultEmpty {
        if PathBuf::from(self.disk_path).exists() {
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// Called on filesystem unmount.
    fn destroy(&self, _req: RequestInfo) {
        // Nothing.
    }

    /// Look up a filesystem entry and get its attributes.
    ///
    /// * `parent`: path to the parent of the entry being looked up
    /// * `name`: the name of the entry (under `parent`) being looked up.
    fn lookup(&self, _req: RequestInfo, _parent: &Path, _name: &OsStr) -> ResultEntry {
        Err(libc::ENOSYS)
    }

    /// Get the attributes of a filesystem entry.
    ///
    /// * `fl`: a FileLike object if this is called on an open file.
    fn getattr(&self,
               _req: RequestInfo,
               _path: &Path,
               _fl: Option<&Self::FileLike>)
               -> ResultGetattr {
        Err(libc::ENOSYS)
    }

    // The following operations in the FUSE C API are all one kernel call: setattr
    // We split them out to match the C API's behavior.

    /// Change the mode of a filesystem entry.
    ///
    /// * `fl`: a FileLike object if this is called on an open file.
    /// * `mode`: the mode to change the file to.
    fn chmod(&self,
             _req: RequestInfo,
             _path: &Path,
             _fl: Option<&Self::FileLike>,
             _mode: u32)
             -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Change the owner UID and/or group GID of a filesystem entry.
    ///
    /// * `fl`: a FileLike object if this is called on an open file.
    /// * `uid`: user ID to change the file's owner to. If `None`, leave the UID unchanged.
    /// * `gid`: group ID to change the file's group to. If `None`, leave the GID unchanged.
    fn chown(&self,
             _req: RequestInfo,
             _path: &Path,
             _fl: Option<&Self::FileLike>,
             _uid: Option<u32>,
             _gid: Option<u32>)
             -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Set the length of a file.
    ///
    /// * `fl`: a FileLike object if this is called on an open file.
    /// * `size`: size in bytes to set as the file's length.
    fn truncate(&self,
                _req: RequestInfo,
                _path: &Path,
                _fl: Option<&Self::FileLike>,
                _size: u64)
                -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Set timestamps of a filesystem entry.
    ///
    /// * `fl`: a FileLike object if this is called on an open file.
    /// * `atime`: the time of last access.
    /// * `mtime`: the time of last modification.
    fn utimens(&self,
               _req: RequestInfo,
               _path: &Path,
               _fl: Option<&Self::FileLike>,
               _atime: Option<Timespec>,
               _mtime: Option<Timespec>)
               -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Set timestamps of a filesystem entry (with extra options only used on MacOS).
    #[allow(unknown_lints, too_many_arguments)]
    fn utimens_macos(&self,
                     _req: RequestInfo,
                     _path: &Path,
                     _fl: Option<&Self::FileLike>,
                     _crtime: Option<Timespec>,
                     _chgtime: Option<Timespec>,
                     _bkuptime: Option<Timespec>,
                     _flags: Option<u32>)
                     -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    // END OF SETATTR FUNCTIONS

    /// Read a symbolic link.
    fn readlink(&self, _req: RequestInfo, _path: &Path) -> ResultData {
        Err(libc::ENOSYS)
    }

    /// Create a special file.
    ///
    /// * `parent`: path to the directory to make the entry under.
    /// * `name`: name of the entry.
    /// * `mode`: mode for the new entry.
    /// * `rdev`: if mode has the bits `S_IFCHR` or `S_IFBLK` set, this is the major and minor
    ///    numbers for the device file. Otherwise it should be ignored.
    fn mknod(&self,
             _req: RequestInfo,
             _parent: &Path,
             _name: &OsStr,
             _mode: u32,
             _rdev: u32)
             -> ResultEntry {
        Err(libc::ENOSYS)
    }

    /// Create a directory.
    ///
    /// * `parent`: path to the directory to make the directory under.
    /// * `name`: name of the directory.
    /// * `mode`: permissions for the new directory.
    fn mkdir(&self, _req: RequestInfo, _parent: &Path, _name: &OsStr, _mode: u32) -> ResultEntry {
        Err(libc::ENOSYS)
    }

    /// Remove a file.
    ///
    /// * `parent`: path to the directory containing the file to delete.
    /// * `name`: name of the file to delete.
    fn unlink(&self, _req: RequestInfo, _parent: &Path, _name: &OsStr) -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Remove a directory.
    ///
    /// * `parent`: path to the directory containing the directory to delete.
    /// * `name`: name of the directory to delete.
    fn rmdir(&self, _req: RequestInfo, _parent: &Path, _name: &OsStr) -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Create a symbolic link.
    ///
    /// * `parent`: path to the directory to make the link in.
    /// * `name`: name of the symbolic link.
    /// * `target`: path (may be relative or absolute) to the target of the link.
    fn symlink(&self,
               _req: RequestInfo,
               _parent: &Path,
               _name: &OsStr,
               _target: &Path)
               -> ResultEntry {
        Err(libc::ENOSYS)
    }

    /// Rename a filesystem entry.
    ///
    /// * `parent`: path to the directory containing the existing entry.
    /// * `name`: name of the existing entry.
    /// * `newparent`: path to the directory it should be renamed into (may be the same as
    ///   `parent`).
    /// * `newname`: name of the new entry.
    fn rename(&self,
              _req: RequestInfo,
              _parent: &Path,
              _name: &OsStr,
              _newparent: &Path,
              _newname: &OsStr)
              -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Create a hard link.
    ///
    /// * `path`: path to an existing file.
    /// * `newparent`: path to the directory for the new link.
    /// * `newname`: name for the new link.
    fn link(&self,
            _req: RequestInfo,
            _path: &Path,
            _newparent: &Path,
            _newname: &OsStr)
            -> ResultEntry {
        Err(libc::ENOSYS)
    }

    /// Open a file.
    ///
    /// * `path`: path to the file.
    /// * `flags`: one of `O_RDONLY`, `O_WRONLY`, or `O_RDWR`, plus maybe additional flags.
    ///
    /// Return a tuple of (file handle, flags). The file handle will be passed to any subsequent
    /// calls that operate on the file, and can be any value you choose, though it should allow
    /// your filesystem to identify the file opened even without any path info.
    fn open(&self, _req: RequestInfo, _path: &Path, _flags: u32) -> ResultOpenObj<Self::FileLike> {
        Err(libc::ENOSYS)
    }

    /// Read from a file.
    ///
    /// Note that it is not an error for this call to request to read past the end of the file, and
    /// you should only return data up to the end of the file (i.e. the number of bytes returned
    /// will be fewer than requested; possibly even zero). Do not extend the file in this case.
    ///
    /// * `path`: path to the file.
    /// * `fl`: FileLike object returned from the `open` call.
    /// * `offset`: offset into the file to start reading.
    /// * `size`: number of bytes to read.
    ///
    /// Return the bytes read.
    fn read(&self,
            _req: RequestInfo,
            _path: &Path,
            _fl: &Self::FileLike,
            _offset: u64,
            _size: u32)
            -> ResultData {
        Err(libc::ENOSYS)
    }

    /// Write to a file.
    ///
    /// * `path`: path to the file.
    /// * `fl`: FileLike object returned from the `open` call.
    /// * `offset`: offset into the file to start writing.
    /// * `data`: the data to write
    /// * `flags`:
    ///
    /// Return the number of bytes written.
    fn write(&self,
             _req: RequestInfo,
             _path: &Path,
             _fl: &Self::FileLike,
             _offset: u64,
             _data: Vec<u8>,
             _flags: u32)
             -> ResultWrite {
        Err(libc::ENOSYS)
    }

    /// Called each time a program calls `close` on an open file.
    ///
    /// Note that because file descriptors can be duplicated (by `dup`, `dup2`, `fork`) this may be
    /// called multiple times for a given file handle. The main use of this function is if the
    /// filesystem would like to return an error to the `close` call. Note that most programs
    /// ignore the return value of `close`, though.
    ///
    /// NOTE: the name of the method is misleading, since (unlike fsync) the filesystem is not
    /// forced to flush pending writes. One reason to flush data, is if the filesystem wants to
    /// return write errors. (Currently unsupported) If the filesystem supports file locking
    /// operations (setlk, getlk) it should remove all locks belonging to 'lock_owner'.
    ///
    /// * `path`: path to the file.
    /// * `fl`: FileLike object returned from the `open` call.
    /// * `lock_owner`: if the filesystem supports locking (`setlk`, `getlk`), remove all locks
    ///   belonging to this lock owner.
    fn flush(&self,
             _req: RequestInfo,
             _path: &Path,
             _fl: &Self::FileLike,
             _lock_owner: u64)
             -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Write out any pending changes of a file.
    ///
    /// When this returns, data should be written to persistent storage.
    ///
    /// * `path`: path to the file.
    /// * `fl`: FileLike object returned from the `open` call.
    /// * `datasync`: if `false`, just write metadata, otherwise also write file data.
    fn fsync(&self,
             _req: RequestInfo,
             _path: &Path,
             _fl: &Self::FileLike,
             _datasync: bool)
             -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Open a directory.
    ///
    /// Analogous to the `opend` call.
    ///
    /// * `path`: path to the directory.
    /// * `flags`: file access flags. Will contain `O_DIRECTORY` at least.
    ///
    /// Return a tuple of (file handle, flags). The file handle will be passed to any subsequent
    /// calls that operate on the directory, and can be any value you choose, though it should
    /// allow your filesystem to identify the directory opened even without any path info.
    fn opendir(&self,
               _req: RequestInfo,
               _path: &Path,
               _flags: u32)
               -> ResultOpenObj<Self::DirLike> {
        Err(libc::ENOSYS)
    }

    /// Get the entries of a directory.
    ///
    /// * `path`: path to the directory.
    /// * `dl`: DirLike object returned from the `opendir` call.
    ///
    /// Return all the entries of the directory.
    fn readdir(&self, _req: RequestInfo, _path: &Path, _dl: &Self::DirLike) -> ResultReaddir {
        Err(libc::ENOSYS)
    }

    /// Write out any pending changes to a directory.
    ///
    /// Analogous to the `fsync` call.
    fn fsyncdir(&self,
                _req: RequestInfo,
                _path: &Path,
                _dl: &Self::DirLike,
                _datasync: bool)
                -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Get filesystem statistics.
    ///
    /// * `path`: path to some folder in the filesystem.
    ///
    /// See the `Statfs` struct for more details.
    fn statfs(&self, _req: RequestInfo, _path: &Path) -> ResultStatfs {
        Err(libc::ENOSYS)
    }

    /// Set a file extended attribute.
    ///
    /// * `path`: path to the file.
    /// * `name`: attribute name.
    /// * `value`: the data to set the value to.
    /// * `flags`: can be either `XATTR_CREATE` or `XATTR_REPLACE`.
    /// * `position`: offset into the attribute value to write data.
    fn setxattr(&self,
                _req: RequestInfo,
                _path: &Path,
                _name: &OsStr,
                _value: &[u8],
                _flags: u32,
                _position: u32)
                -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Get a file extended attribute.
    ///
    /// * `path`: path to the file
    /// * `name`: attribute name.
    /// * `size`: the maximum number of bytes to read.
    ///
    /// If `size` is 0, return `Xattr::Size(n)` where `n` is the size of the attribute data.
    /// Otherwise, return `Xattr::Data(data)` with the requested data.
    fn getxattr(&self, _req: RequestInfo, _path: &Path, _name: &OsStr, _size: u32) -> ResultXattr {
        Err(libc::ENOSYS)
    }

    /// List extended attributes for a file.
    ///
    /// * `path`: path to the file.
    /// * `size`: maximum number of bytes to return.
    ///
    /// If `size` is 0, return `Xattr::Size(n)` where `n` is the size required for the list of
    /// attribute names.
    /// Otherwise, return `Xattr::Data(data)` where `data` is all the null-terminated attribute
    /// names.
    fn listxattr(&self, _req: RequestInfo, _path: &Path, _size: u32) -> ResultXattr {
        Err(libc::ENOSYS)
    }

    /// Remove an extended attribute for a file.
    ///
    /// * `path`: path to the file.
    /// * `name`: name of the attribute to remove.
    fn removexattr(&self, _req: RequestInfo, _path: &Path, _name: &OsStr) -> ResultEmpty {
        Err(libc::ENOSYS)
    }

    /// Check for access to a file.
    ///
    /// * `path`: path to the file.
    /// * `mask`: mode bits to check for access to.
    ///
    /// Return `Ok(())` if all requested permissions are allowed, otherwise return `Err(EACCES)`
    /// or other error code as appropriate (e.g. `ENOENT` if the file doesn't exist).
    fn access(&self, _req: RequestInfo, _path: &Path, _mask: u32) -> ResultEmpty {
        Err(libc::ENOSYS)
    }

}
