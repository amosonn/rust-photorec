#[cfg(test)]
#[macro_use]
mod tests;

mod file_description;
mod reader;
mod report;
mod segment_tree;
mod segment_array_tree;

pub use crate::file_description::{ByteRun, FileDescription, FileDescriptionPos, FileDescriptionError};
pub use crate::reader::ByteRunsReader;
pub use crate::report::{ReportXml, ReportXmlError};
pub use crate::segment_tree::{Segment, SegmentTree, SegmentTreeError, Entry, VacantEntry, OccupiedEntry};
pub use crate::segment_array_tree::{SegmentArrayTree, SegmentArrayTreeError, AddStatus};

#[cfg(test)]
#[macro_use]
extern crate quote;
