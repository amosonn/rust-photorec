#[cfg(test)]
#[macro_use]
mod tests;

mod byte_runs;
mod reader;
mod report;
mod segment_tree;

pub use crate::byte_runs::{ByteRun, FileDescription, FileDescriptionPos, FileDescriptionError};
pub use crate::reader::ByteRunsReader;
pub use crate::report::{ReportXml, ReportXmlError};
pub use crate::segment_tree::{Segment, SegmentTree, Entry, VacantEntry, OccupiedEntry, Get, GetMut, Insert, Contains};

#[cfg(test)]
#[macro_use]
extern crate quote;
