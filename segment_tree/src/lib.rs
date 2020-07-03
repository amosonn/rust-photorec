mod segment_tree;
mod segment_array_tree;

pub use crate::segment_tree::{Segment, SegmentTree, SegmentTreeError, Entry, VacantEntry, OccupiedEntry};
pub use crate::segment_array_tree::{SegmentArrayTree, SegmentArrayTreeError, AddStatus};

#[cfg(test)]
#[macro_use]
extern crate assert_let;
