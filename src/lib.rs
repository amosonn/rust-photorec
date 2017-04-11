
extern crate xmltree;

#[cfg(feature = "filesystem")]
extern crate fuse_fl;
#[cfg(feature = "filesystem")]
extern crate serde;
#[cfg(feature = "filesystem")]
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod tests;

mod byte_runs;
mod reader;
mod report;

#[cfg(feature = "filesystem")]
mod reader_at;
#[cfg(feature = "filesystem")]
mod filesystem;


pub use byte_runs::{ByteRun, ByteRunsRef, ByteRunsRefPos, ByteRunsRefError};
pub use reader::ByteRunsReader;
pub use report::{ReportXml, ReportXmlError};

#[cfg(feature = "filesystem")]
pub use reader_at::ByteRunsReaderAt;

#[test]
fn it_works() {
    let y = Some(3);
    assert_let!(Some(x) = y, { assert_eq!(x, 3) });
}
