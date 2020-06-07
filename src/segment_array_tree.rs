
use super::segment_tree::{Segment, SegmentTree, SegmentTreeError, Entry};

use std::marker::PhantomData;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SegmentArrayTreeError {
    #[error("Got error in underlaying SegmentTree: {0}")]
    SegmentTreeError(#[from] SegmentTreeError),
    #[error("A SegmentArray intersected with several disjoint SegmentArray-s")]
    OverlappingSegmentArrays,
    #[error("Two intersecting SegmentArray-s, without one being a strict superset of the other")]
    IncompatibleSegmentArrays,
}

type Result<T> = std::result::Result<T, SegmentArrayTreeError>;

pub struct SegmentArrayTree<M, I> {
    tree: SegmentTree<usize>,
    descriptions: Vec<M>,
    _phantom: PhantomData<*const I>,
}

impl<M, I> SegmentArrayTree<M, I> where M: AsRef<[I]>, for<'a> &'a I: Into<Segment> + Eq {
    pub fn new() -> Self {
        SegmentArrayTree {
            tree: SegmentTree::new(),
            descriptions: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn add(&mut self, mut desc: M) -> Result<Option<M>>  {
        let mut idx: Option<usize> = None;
        for seg in desc.as_ref().into_iter().map(|s| s.into()) {
            if let Some(x) = self.tree.get_segment(seg)? {
                if idx.get_or_insert(*x) != x {
                    return Err(SegmentArrayTreeError::OverlappingSegmentArrays);
                }
            }
        }

        let (idx, ret) = match idx {
            None => {
                self.descriptions.push(desc);
                (Some(self.descriptions.len() - 1), None)
            }
            Some(x) => {
                // Make sure they are really compatible
                for (br1, br2) in desc.as_ref().into_iter().zip(self.descriptions[x].as_ref().into_iter()) {
                    if br1 != br2 { return Err(SegmentArrayTreeError::IncompatibleSegmentArrays); }
                }
                // If the new one is larger, we insert it and return the old one
                let idx = if desc.as_ref().into_iter().len() > self.descriptions[x].as_ref().into_iter().len() {
                    std::mem::swap(&mut desc, &mut self.descriptions[x]);
                    Some(x)
                // Else, we don't need to add any segments to the tree
                } else { None };
                (idx, Some(desc))
            }
        };

        if let Some(idx) = idx {
            for seg in self.descriptions[idx].as_ref().into_iter().map(|s| s.into()) {
                if let Entry::Vacant(entry) = self.tree.entry_segment(seg)? {
                    entry.insert(idx);
                }
            }
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::{SegmentArrayTree, SegmentArrayTreeError};
    use crate::file_description::{ByteRun, FileDescription};

    #[test]
    fn smoke() {
        let mut fdt: SegmentArrayTree<FileDescription, ByteRun> = SegmentArrayTree::new();
    }
}
