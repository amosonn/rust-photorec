
use super::file_description::{ByteRun, FileDescription};
use super::segment_tree::{Segment, SegmentTree, SegmentTreeError, Entry};

use thiserror::Error;

impl From<&ByteRun> for Segment {
    fn from(br: &ByteRun) -> Self {
        Segment { start: br.disk_pos, end: br.disk_pos + br.len }
    }
}

#[derive(Debug, Error)]
pub enum FileDescriptionTreeError {
    #[error("Got error in underlaying SegmentTree: {0}")]
    SegmentTreeError(#[from] SegmentTreeError),
    #[error("A FileDescription intersected with several disjoint FileDescription-s")]
    OverlappingFileDecriptions,
    #[error("Two intersecting FileDescription-s, without one being a strict superset of the other")]
    IncompatibleFileDescriptions,
}

type Result<T> = std::result::Result<T, FileDescriptionTreeError>;

pub struct FileDescriptionTree {
    tree: SegmentTree<usize>,
    descriptions: Vec<FileDescription>,
}

impl FileDescriptionTree {
    pub fn new() -> Self {
        FileDescriptionTree {
            tree: SegmentTree::new(),
            descriptions: Vec::new(),
        }
    }

    pub fn add(&mut self, mut desc: FileDescription) -> Result<Option<FileDescription>>  {
        let mut idx: Option<usize> = None;
        for seg in desc.iter().map(|br| br.into()) {
            if let Some(x) = self.tree.get_segment(seg)? {
                if idx.get_or_insert(*x) != x {
                    return Err(FileDescriptionTreeError::OverlappingFileDecriptions);
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
                for (br1, br2) in desc.iter().zip(self.descriptions[x].iter()) {
                    if br1 != br2 { return Err(FileDescriptionTreeError::IncompatibleFileDescriptions); }
                }
                // If the new one is larger, we insert it and return the old one
                let idx = if desc.iter().len() > self.descriptions[x].iter().len() {
                    std::mem::swap(&mut desc, &mut self.descriptions[x]);
                    Some(x)
                // Else, we don't need to add any segments to the tree
                } else { None };
                (idx, Some(desc))
            }
        };

        if let Some(idx) = idx {
            for seg in self.descriptions[idx].iter().map(|br| br.into()) {
                if let Entry::Vacant(entry) = self.tree.entry_segment(seg)? {
                    entry.insert(idx);
                }
            }
        }

        Ok(ret)
    }
}
