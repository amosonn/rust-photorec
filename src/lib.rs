
#[macro_use]
mod tests;

mod byte_runs;
mod reader;
mod report;

pub use byte_runs::{ByteRun, ByteRunsRef};
pub use reader::ByteRunsReader;

extern crate xmltree;

#[test]
fn it_works() {
    let y = Some(3);
    assert_let!(Some(x) = y, { assert_eq!(x, 3) });
}
