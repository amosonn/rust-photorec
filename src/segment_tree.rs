use std::collections::btree_map::Entry as BEntry;
use std::collections::BTreeMap;
use std::mem;
use core::ops::{RangeBounds, Bound};

use thiserror::Error;

use Entry::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone)]
pub struct Segment<T> {
    pub start: T,
    pub end: T,
}

impl From<&Segment<u64>> for Segment<u64> {
    fn from(seg: &Segment<u64>) -> Self { seg.clone() }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SegmentTreeError {
    #[error("Requested segment intersects one in the tree, at point {0}")]
    Intersect(u64),
}

type Result<T> = std::result::Result<T, SegmentTreeError>;

type BTree<V> = BTreeMap<u64, SegmentValue<V>>;

#[derive(Clone, Debug)]
pub struct SegmentTree<V>(BTree<V>);

struct RefRangeInclusive<'a, T: 'a> {
    start: &'a T,
    end: &'a T,
}

impl<'a, T> RangeBounds<T> for RefRangeInclusive<'a, T> {
    fn start_bound(&self) -> Bound<&T> { Bound::Included(self.start) }
    fn end_bound(&self) -> Bound<&T> { Bound::Included(self.end) }
}

impl<T: PartialOrd> Segment<T> {
    pub fn new(start: T, end: T) -> Segment<T> { assert!(start < end); Segment { start, end } }
}

impl<'a, T: PartialOrd> Segment<T> {
    fn get_range(&'a self) -> impl RangeBounds<T> + 'a { RefRangeInclusive { start: &self.start, end: &self.end } }
}

/// The value for a SegmentTree<V>
#[derive(Clone, Debug, PartialEq, Eq)]
enum SegmentValue<V> {
    /// This is the start of a segment
    Start,
    /// This is the end of a segment, we store the real value here
    End(V),
    /// This is the start of one segment and the end of another
    EndStart(V),
}

impl<V> SegmentValue<V> {
    fn get_ref<'a>(&'a self) -> Option<&'a V> {
        match self {
            SegmentValue::Start => None,
            SegmentValue::End(ref t) | SegmentValue::EndStart(ref t) => Some(t)
        }
    }

    fn get_mut<'a>(&'a mut self) -> Option<&'a mut V> {
        match self {
            SegmentValue::Start => None,
            SegmentValue::End(ref mut t) | SegmentValue::EndStart(ref mut t) => Some(t)
        }
    }
}

type InnerEntry<'a, V> = BEntry<'a, u64, SegmentValue<V>>;

fn remove_start<V>(tree: &mut BTree<V>, start: &u64) {
    let val = tree.get_mut(start).unwrap();
    let remove: bool = if let SegmentValue::Start = val { true } else { false };
    if remove {
        tree.remove(start);
    } else {
        let mut owned = SegmentValue::Start;
        // This way we can "steal" the inner V
        mem::swap(val, &mut owned);
        let v: V = match owned {
            SegmentValue::Start => panic!("We just checked this isn't Start..."),
            SegmentValue::End(_) => panic!("Expected Start/EndStart"),
            SegmentValue::EndStart(v) => v,
        };
        let mut owned = SegmentValue::End(v);
        mem::swap(val, &mut owned);
    }
}

fn remove_end<V>(tree: &mut BTree<V>, end: &u64) -> V {
    let val = tree.get_mut(end).unwrap();
    let remove: bool = if let SegmentValue::End(_) = val { true } else { false };
    if remove {
        if let SegmentValue::End(v) = tree.remove(end).unwrap() { v } else { panic!("We just checked this is End") }
    } else {
        let mut owned = SegmentValue::Start;
        mem::swap(val, &mut owned);
        match owned {
            SegmentValue::Start => panic!("Expected End/EndStart"),
            SegmentValue::End(_) => panic!("We just checked this isn't End"),
            SegmentValue::EndStart(v) => v,
        }
    }
}

fn remove<V>(tree: &mut BTree<V>, seg: &Segment<u64>) -> V {
    remove_start(tree, &seg.start);
    remove_end(tree, &seg.end)
}

fn add_start<'a, V>(entry: InnerEntry<'a, V>) {
    match entry {
        BEntry::Vacant(entry) => { entry.insert(SegmentValue::Start); }
        BEntry::Occupied(entry) => {
            let entry = entry.into_mut();
            let mut owned = SegmentValue::Start;
            // This way we can "steal" the inner V
            mem::swap(entry, &mut owned);
            let v: V = match owned {
                SegmentValue::Start | SegmentValue::EndStart(_) => panic!("Didn't expect Start/EndStart"),
                SegmentValue::End(v) => v,
            };
            let mut owned = SegmentValue::EndStart(v);
            mem::swap(entry, &mut owned);
        },
    }
}

fn add_end<'a, V>(entry: InnerEntry<'a, V>, v: V) -> &'a mut V {
    match entry {
        BEntry::Vacant(entry) => entry.insert(SegmentValue::End(v)),
        BEntry::Occupied(entry) => {
            let entry = entry.into_mut();
            let mut owned = SegmentValue::EndStart(v);
            mem::swap(entry, &mut owned);
            match owned {
                SegmentValue::End(_) | SegmentValue::EndStart(_) => panic!("Didn't expect End/EndStart"),
                SegmentValue::Start => {},
            };
            entry
        },
    }.get_mut().unwrap()
}

#[derive(Debug)]
pub enum Entry<'a, V> {
    Vacant(VacantEntry<'a, V>),
    Occupied(OccupiedEntry<'a, V>),
}

#[derive(Debug)]
pub struct VacantEntry<'a, V> {
    tree: &'a mut SegmentTree<V>,
    seg: Segment<u64>,
}

#[derive(Debug)]
pub struct OccupiedEntry<'a, V> {
    tree: &'a mut SegmentTree<V>,
    seg: Segment<u64>,
}

impl<'a, V> Entry<'a, V> {
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default),
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(default()),
        }
    }

    pub fn key(&self) -> &Segment<u64> {
        match *self {
            Occupied(ref entry) => entry.key(),
            Vacant(ref entry) => entry.key(),
        }
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Occupied(mut entry) => {
                f(entry.get_mut());
                Occupied(entry)
            }
            Vacant(entry) => Vacant(entry),
        }
    }
}

impl<'a, V: Default> Entry<'a, V> {
    pub fn or_default(self) -> &'a mut V {
        match self {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

impl<'a, V> VacantEntry<'a, V> {
    pub fn key(&self) -> &Segment<u64> {
        &self.seg
    }

    pub fn into_key(self) -> Segment<u64> {
        self.seg
    }

    pub fn insert(self, value: V) -> &'a mut V {
        add_start(self.tree.0.entry(self.seg.start));
        add_end(self.tree.0.entry(self.seg.end), value)
    }
}

impl<'a, V> OccupiedEntry<'a, V> {
    pub fn key(&self) -> &Segment<u64> {
        &self.seg
    }

    pub fn remove_entry(mut self) -> (Segment<u64>, V) {
        let tmp = self.remove_impl();
        (self.seg, tmp)
    }

    pub fn get(&self) -> &V {
        self.tree.0.get(&self.seg.end).unwrap().get_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut V {
        self.tree.0.get_mut(&self.seg.end).unwrap().get_mut().unwrap()
    }

    pub fn into_mut(self) -> &'a mut V {
        self.tree.0.get_mut(&self.seg.end).unwrap().get_mut().unwrap()
    }

    pub fn insert(&mut self, value: V) -> V {
        mem::replace(self.get_mut(), value)
    }

    pub fn remove(mut self) -> V {
        self.remove_impl()
    }

    fn remove_impl(&mut self) -> V {
        remove(&mut self.tree.0, &self.seg)
    }
}

macro_rules! impl_segment {
    { $iter:expr, $seg:expr, $missing:expr, $found:expr } => {
        let mut iter = $iter;
        match iter.next() {
            None => $missing,
            Some((start, SegmentValue::Start)) |
            Some((start, SegmentValue::EndStart(_))) if start == &$seg.start => {
                match iter.next() {
                    Some((end, SegmentValue::End(v))) |
                    Some((end, SegmentValue::EndStart(v))) if end == &$seg.end => {
                        match iter.next() {
                            None => $found(v),
                            _ => panic!("range should not contain nodes after end"),
                        }
                    }
                    _ => Err(SegmentTreeError::Intersect(*start)),
                }
            },
            Some((start, SegmentValue::End(_))) if start == &$seg.start => {
                match iter.next() {
                    None => $missing,
                    Some((end, SegmentValue::Start)) if end == &$seg.end => {
                        match iter.next() {
                            None => $missing,
                            _ => panic!("range should not contain nodes after end"),
                        }
                    }
                    Some((point, _)) => Err(SegmentTreeError::Intersect(*point)),
                }
            }
            Some((end, SegmentValue::Start)) if end == &$seg.end => {
                match iter.next() {
                    None => $missing,
                    _ => panic!("range should not contain nodes after end"),
                }
            }
            Some((_, SegmentValue::End(_))) |
            Some((_, SegmentValue::EndStart(_)))  => Err(SegmentTreeError::Intersect($seg.start)),
            Some((start, SegmentValue::Start)) => Err(SegmentTreeError::Intersect(*start)),
        }
    }
}

struct RangeFromNonInclusive(u64);

impl RangeBounds<u64> for RangeFromNonInclusive {
    fn start_bound(&self) -> Bound<&u64> { Bound::Excluded(&self.0) }
    fn end_bound(&self) -> Bound<&u64> { Bound::Unbounded }
}

impl<V> SegmentTree<V> {
    pub fn new() -> Self { SegmentTree(BTreeMap::new()) }

    pub fn get_segment(&self, seg: &Segment<u64>) -> Result<Option<&V>> {
        impl_segment! { self.0.range(seg.get_range()), seg, Ok(None), |v| Ok(Some(v)) }
    }

    pub fn get_mut_segment(&mut self, seg: &Segment<u64>) -> Result<Option<&mut V>> {
        impl_segment! { self.0.range_mut(seg.get_range()), seg, Ok(None), |v| Ok(Some(v)) }
    }

    pub fn contains_segment(&self, seg: &Segment<u64>) -> Result<bool> {
        impl_segment! { self.0.range(seg.get_range()), seg, Ok(false), |_| Ok(true) }
    }

    /// Gets an Ok(Entry), Vacant or Occupied, if the tree doesn't contain any intersection with the
    /// segment or contains it exactly. Returns None otherwise.
    pub fn entry_segment(&mut self, seg: Segment<u64>) -> Result<Entry<V>> {
        Ok(if self.contains_segment(&seg)? {
            Entry::Occupied(OccupiedEntry { tree: self, seg })
        } else {
            Entry::Vacant(VacantEntry { tree: self, seg })
        })
    }

    pub fn insert_segment(&mut self, seg: Segment<u64>, value: V) -> std::result::Result<Option<V>, (V, SegmentTreeError)> {
        match self.entry_segment(seg) {
            Ok(Entry::Vacant(entry)) => {
                entry.insert(value);
                Ok(None)
            },
            Ok(Entry::Occupied(mut entry)) => Ok(Some(entry.insert(value))),
            Err(e) => Err((value, e)),
        }
    }

    pub fn remove_segment(&mut self, seg: &Segment<u64>) -> Result<Option<V>> {
        Ok(if self.contains_segment(seg)? {
            Some(remove(&mut self.0, seg))
        } else {
            None
        })
    }

    pub fn get_containing_segment(&self, point: u64) -> Option<(Segment<u64>, &V)> {
        let (start_idx, start_val) = self.0.range(..=point).next_back()?;
        let (end_idx, end_val) = self.0.range(RangeFromNonInclusive(point)).next()?;
        if let SegmentValue::End(_) = start_val { return None; }
        let val = match end_val {
            SegmentValue::End(x) | SegmentValue::EndStart(x) => x,
            SegmentValue::Start => { return None; },
        };
        Some((Segment { start: *start_idx, end: *end_idx }, val))
    }
}

#[cfg(test)]
mod tests {
    use super::{Segment, SegmentTree, SegmentTreeError, Entry};
    #[test]
    fn smoke() {
        #[derive(Debug, PartialEq, Eq)]
        struct X(u64);

        let mut st = SegmentTree::new();
        assert_eq!(st.get_segment(&Segment::new(1, 3)), Ok(None));
        assert_eq!(st.get_mut_segment(&Segment::new(1, 3)), Ok(None));
        assert_eq!(st.contains_segment(&Segment::new(1, 3)), Ok(false));
        assert_eq!(st.insert_segment(Segment::new(1, 3), X(0)), Ok(None));
        assert_eq!(st.get_segment(&Segment::new(1, 3)), Ok(Some(&X(0))));
        assert_eq!(st.get_mut_segment(&Segment::new(1, 3)), Ok(Some(&mut X(0))));
        assert_eq!(st.contains_segment(&Segment::new(1, 3)), Ok(true));
        // TODO: these are brittle, we should check for ranges
        assert_eq!(st.contains_segment(&Segment::new(2, 3)), Err(SegmentTreeError::Intersect(2)));
        assert_eq!(st.contains_segment(&Segment::new(2, 4)), Err(SegmentTreeError::Intersect(2)));
        assert_eq!(st.contains_segment(&Segment::new(0, 2)), Err(SegmentTreeError::Intersect(1)));
        assert_eq!(st.contains_segment(&Segment::new(0, 4)), Err(SegmentTreeError::Intersect(1)));
        assert_eq!(st.contains_segment(&Segment::new(3, 6)), Ok(false));
        assert_eq!(st.contains_segment(&Segment::new(0, 1)), Ok(false));
        assert_eq!(st.insert_segment(Segment::new(7, 9), X(1)), Ok(None));
        assert_eq!(st.insert_segment(Segment::new(1, 5), X(2)), Err((X(2), SegmentTreeError::Intersect(1))));
        assert_eq!(st.insert_segment(Segment::new(1, 3), X(3)), Ok(Some(X(0))));
        assert_eq!(st.insert_segment(Segment::new(3, 4), X(4)), Ok(None));
        assert_eq!(st.contains_segment(&Segment::new(4, 7)), Ok(false));
        assert_let!(Ok(Entry::Vacant(entry)) = st.entry_segment(Segment::new(5, 7)), {
            assert_eq!(entry.insert(X(5)), &mut X(5));
        });
        assert_eq!(st.insert_segment(Segment::new(4, 5), X(6)), Ok(None));
        assert_eq!(st.get_segment(&Segment::new(1, 3)), Ok(Some(&X(3))));
        assert_eq!(st.get_segment(&Segment::new(3, 4)), Ok(Some(&X(4))));
        assert_eq!(st.get_segment(&Segment::new(4, 5)), Ok(Some(&X(6))));
        assert_eq!(st.get_segment(&Segment::new(5, 7)), Ok(Some(&X(5))));
        assert_eq!(st.get_segment(&Segment::new(7, 9)), Ok(Some(&X(1))));
        assert_let!(Ok(Entry::Occupied(entry)) = st.entry_segment(Segment::new(4, 5)), {
            assert_eq!(entry.remove(), X(6));
        });
        assert_eq!(st.get_segment(&Segment::new(4, 5)), Ok(None));
        assert_let!(Ok(Entry::Occupied(mut entry)) = st.entry_segment(Segment::new(5, 7)), {
            assert_eq!(entry.insert(X(7)), X(5));
        });
        assert_let!(Err(SegmentTreeError::Intersect(1)) = st.entry_segment(Segment::new(0, 9)));
        assert_eq!(st.remove_segment(&Segment::new(0, 9)), Err(SegmentTreeError::Intersect(1)));
        assert_eq!(st.remove_segment(&Segment::new(4, 5)), Ok(None));
        assert_eq!(st.remove_segment(&Segment::new(5, 7)), Ok(Some(X(7))));
        assert_eq!(st.get_containing_segment(0), None);
        assert_eq!(st.get_containing_segment(1), Some((Segment::new(1, 3), &X(3))));
        assert_eq!(st.get_containing_segment(2), Some((Segment::new(1, 3), &X(3))));
        assert_eq!(st.get_containing_segment(3), Some((Segment::new(3, 4), &X(4))));
        assert_eq!(st.get_containing_segment(4), None);
        assert_eq!(st.get_containing_segment(5), None);
        assert_eq!(st.get_containing_segment(7), Some((Segment::new(7, 9), &X(1))));
        assert_eq!(st.get_containing_segment(9), None);
        assert_eq!(st.get_containing_segment(10), None);
    }
}
