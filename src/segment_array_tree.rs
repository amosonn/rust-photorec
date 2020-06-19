
use super::segment_tree::{Segment, SegmentTree, SegmentTreeError, Entry};

use std::marker::PhantomData;
use std::slice;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SegmentArrayTreeError {
    #[error("Intersected a segment with segment array at index {0}")]
    IntersectingSegment(usize),
    #[error("A SegmentArray overlapped with several disjoint SegmentArray-s, at least at indexes {0}, {1}")]
    OverlappingSegmentArrays(usize, usize),
    #[error("Overlapped with SegmentArray at index {0}, without one being a strict extension of the other")]
    IncompatibleSegmentArrays(usize),
}

#[derive(Debug, Clone)]
pub struct SegmentArrayTree<M, I> {
    tree: SegmentTree<u64, usize>,
    segment_arrays: Vec<M>,
    _phantom: PhantomData<*const I>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum AddStatus<M> {
    /// The segment array was added, didn't intersect with any existing one
    Added,
    /// The tree already contained a larger or equal segment array, here is the argument back
    AlreadyContained(M),
    /// The segment array extended an older one, here it is
    Replaced(M),
}

impl<M, I> SegmentArrayTree<M, I> where M: AsRef<[I]>, for<'a> &'a I: Into<Segment<u64>> + Eq {
    pub fn new() -> Self {
        SegmentArrayTree {
            tree: SegmentTree::new(),
            segment_arrays: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn search_intersecting(&mut self, seg_arr: &M) -> Result<Option<usize>, SegmentArrayTreeError> {
        let mut idx: Option<usize> = None;
        for seg in seg_arr.as_ref().into_iter().map(|s| s.into()) {
            match self.tree.get_segment(&seg) {
                Ok(None) => { continue; },
                Ok(Some(x)) => {
                    if idx.get_or_insert(*x) != x {
                        return Err(SegmentArrayTreeError::OverlappingSegmentArrays(idx.unwrap(), *x));
                    }
                },
                Err(SegmentTreeError::Intersect(point)) => {
                    return Err(SegmentArrayTreeError::IntersectingSegment(*self.tree.get_containing_segment(&point).unwrap().1));
                },
            }
        }
        Ok(idx)
    }

    pub fn add(&mut self, mut seg_arr: M) -> Result<AddStatus<M>, (M, SegmentArrayTreeError)>  {
        let idx = match self.search_intersecting(&seg_arr) {
            Ok(idx) => idx,
            Err(e) => { return Err((seg_arr, e)); },
        };

        let (idx, ret) = match idx {
            None => {
                self.segment_arrays.push(seg_arr);
                (Some(self.segment_arrays.len() - 1), AddStatus::Added)
            }
            Some(x) => {
                // Make sure they are really compatible
                for (br1, br2) in seg_arr.as_ref().into_iter().zip(self.segment_arrays[x].as_ref().into_iter()) {
                    if br1 != br2 { return Err((seg_arr, SegmentArrayTreeError::IncompatibleSegmentArrays(x))); }
                }
                // If the new one is larger, we insert it and return the old one
                if seg_arr.as_ref().into_iter().len() > self.segment_arrays[x].as_ref().into_iter().len() {
                    std::mem::swap(&mut seg_arr, &mut self.segment_arrays[x]);
                    (Some(x), AddStatus::Replaced(seg_arr))
                // Else, we don't need to add any segments to the tree
                } else {
                    (None, AddStatus::AlreadyContained(seg_arr))
                }
            }
        };

        // Some(idx) means that the segment array in idx is new
        if let Some(idx) = idx {
            for seg in self.segment_arrays[idx].as_ref().into_iter().map(|s| s.into()) {
                // We already checked that all the segments are ok
                if let Entry::Vacant(entry) = self.tree.entry_segment(seg).unwrap() {
                    entry.insert(idx);
                }
            }
        }

        Ok(ret)
    }

    pub fn get_by_idx(&self, idx: usize) -> &M { &self.segment_arrays[idx] }

    pub fn iter<'a>(&'a self) -> slice::Iter<'a, M> { self.segment_arrays.iter() }
}

#[cfg(test)]
mod tests {
    use super::{SegmentArrayTree, SegmentArrayTreeError, AddStatus};
    use crate::segment_tree::Segment;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SegmentVecAndInt {
        pub vec: Vec<Segment<u64>>,
        pub num: u64,
    }

    impl AsRef<[Segment<u64>]> for SegmentVecAndInt {
        fn as_ref(&self) -> &[Segment<u64>] {
            &self.vec.as_ref()
        }
    }

    fn build(vec: Vec<(u64, u64)>, num: u64) -> SegmentVecAndInt {
        SegmentVecAndInt {
            vec: vec.into_iter().map(|(start, end)| Segment { start, end } ).collect(),
            num
        }
    }

    #[test]
    fn smoke() {
        let mut sat = SegmentArrayTree::new();
        assert_eq!(sat.add(build(vec![(1, 3), (7, 10), (13, 15)], 0)), Ok(AddStatus::Added));
        assert_eq!(sat.add(build(vec![(1, 3), (7, 10), (13, 15), (17, 18)], 10)), Ok(AddStatus::Replaced(build(vec![(1, 3), (7, 10), (13, 15)], 0))));
        assert_eq!(sat.add(build(vec![(1, 3), (7, 10), (13, 15), (17, 18)], 20)), Ok(AddStatus::AlreadyContained(build(vec![(1, 3), (7, 10), (13, 15), (17, 18)], 20))));
        assert_let!(Ok(Some(i)) = sat.search_intersecting(&build(vec![(1, 3), (13, 15), (20, 22)], 25)), {
            assert_eq!(sat.get_by_idx(i).num, 10);
        });
        assert_eq!(sat.add(build(vec![(3, 6), (10, 13), (16, 17)], 30)), Ok(AddStatus::Added));
        assert_let!(Err(SegmentArrayTreeError::OverlappingSegmentArrays(i1, i2)) = sat.search_intersecting(&build(vec![(1, 3), (10, 13)], 40)), {
            assert_eq!(sat.get_by_idx(i1).num, 10);
            assert_eq!(sat.get_by_idx(i2).num, 30);
        });
        assert_let!(Err((sv, SegmentArrayTreeError::OverlappingSegmentArrays(i1, i2))) = sat.add(build(vec![(1, 3), (10, 13)], 40)), {
            assert_eq!(sv.num, 40);
            assert_eq!(sat.get_by_idx(i1).num, 10);
            assert_eq!(sat.get_by_idx(i2).num, 30);
        });
        assert_let!(Err((sv, SegmentArrayTreeError::IntersectingSegment(i))) = sat.add(build(vec![(2, 4)], 50)), {
            assert_eq!(sv.num, 50);
            assert_eq!(sat.get_by_idx(i).num, 10);
        });
        assert_let!(Err((sv, SegmentArrayTreeError::IncompatibleSegmentArrays(i))) = sat.add(build(vec![(3, 6), (16, 17)], 60)), {
            assert_eq!(sv.num, 60);
            assert_eq!(sat.get_by_idx(i).num, 30);
        });
        assert_let!(Err((sv, SegmentArrayTreeError::IncompatibleSegmentArrays(i))) = sat.add(build(vec![(3, 6), (10, 13), (18, 19)], 70)), {
            assert_eq!(sv.num, 70);
            assert_eq!(sat.get_by_idx(i).num, 30);
        });
        assert_eq!(sat.iter().map(|sv| sv.num).collect::<HashSet<u64>>(), HashSet::<u64>::from_iter(vec![10, 30]));
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RichSegment {
        pub start: u64,
        pub end: u64,
        pub extra: char,
    }

    impl From<&RichSegment> for Segment<u64> {
        fn from(rs: &RichSegment) -> Self {
            Segment {
                start: rs.start,
                end: rs.end
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RichSegmentVecAndInt {
        pub vec: Vec<RichSegment>,
        pub num: u64,
    }

    impl AsRef<[RichSegment]> for RichSegmentVecAndInt {
        fn as_ref(&self) -> &[RichSegment] {
            &self.vec.as_ref()
        }
    }

    fn rich_build(vec: Vec<(u64, u64, char)>, num: u64) -> RichSegmentVecAndInt {
        RichSegmentVecAndInt {
            vec: vec.into_iter().map(|(start, end, extra)| RichSegment { start, end, extra } ).collect(),
            num
        }
    }

    #[test]
    fn test_elem_comparison() {
        let mut sat = SegmentArrayTree::new();
        assert_eq!(sat.add(rich_build(vec![(1, 3, 'a'), (7, 10, 'b'), (13, 15, 'c')], 0)), Ok(AddStatus::Added));
        assert_let!(Err((sv, SegmentArrayTreeError::IncompatibleSegmentArrays(i))) = sat.add(rich_build(vec![(1, 3, 'a'), (7, 10, 'd'), (13, 15, 'c')], 1)), {
            assert_eq!(sv.num, 1);
            assert_eq!(sat.get_by_idx(i).num, 0);
        });

    }
}
