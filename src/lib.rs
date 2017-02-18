#[test]
fn it_works() {
}

mod byte_runs;
mod reader;
mod report;

pub use byte_runs::{ByteRun, ByteRunsRef};
pub use reader::ByteRunsReader;

extern crate xmltree;
