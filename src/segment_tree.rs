use std::collections::btree_map::Entry as BEntry;
use std::collections::BTreeMap;
use std::mem;
use core::ops::RangeInclusive;

use thiserror::Error;

use Entry::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct Segment {
    pub start: u64,
    pub end: u64,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SegmentTreeError {
    #[error("Requested segment intersects one in the tree")]
    Intersect,
}

type Result<T> = std::result::Result<T, SegmentTreeError>;

#[derive(Clone, Debug)]
pub struct SegmentTree<T>(BTreeMap<u64, SegmentValue<T>>);

impl Segment {
    pub fn new(start: u64, end: u64) -> Segment { assert!(start < end); Segment { start, end } }
    fn get_range(&self) -> RangeInclusive<u64> { self.start..=self.end }
}

/// The value for a SegmentTree<T>
#[derive(Clone, Debug, PartialEq, Eq)]
enum SegmentValue<T> {
    /// This is the start of a segment
    Start,
    /// This is the end of a segment, we store the real value here
    End(T),
    /// This is the start of one segment and the end of another
    StartEnd(T),
}

impl<T> SegmentValue<T> {
    fn get_ref<'a>(&'a self) -> Option<&'a T> {
        match self {
            SegmentValue::Start => None,
            SegmentValue::End(ref t) | SegmentValue::StartEnd(ref t) => Some(t)
        }
    }

    fn get_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        match self {
            SegmentValue::Start => None,
            SegmentValue::End(ref mut t) | SegmentValue::StartEnd(ref mut t) => Some(t)
        }
    }
}

type InnerEntry<'a, V> = BEntry<'a, u64, SegmentValue<V>>;

fn remove_start<'a, V>(entry: InnerEntry<'a, V>) {
    match entry {
        BEntry::Vacant(_entry) => panic!("Didn't expect vacant value"),
        BEntry::Occupied(entry) => {
            let remove: bool = if let SegmentValue::Start = entry.get() { true } else { false };
            if remove {
                entry.remove();
            } else {
                let entry = entry.into_mut();
                let mut owned = SegmentValue::Start;
                // This way we can "steal" the inner V
                mem::swap(entry, &mut owned);
                let v: V = match owned {
                    SegmentValue::Start => panic!("We just checked this isn't Start..."),
                    SegmentValue::End(_) => panic!("Expected Start/StartEnd"),
                    SegmentValue::StartEnd(v) => v,
                };
                let mut owned = SegmentValue::End(v);
                mem::swap(entry, &mut owned);
            }
        },
    }
}

fn remove_end<'a, V>(entry: InnerEntry<'a, V>) -> V {
    match entry {
        BEntry::Vacant(_entry) => panic!("Didn't expect vacant value"),
        BEntry::Occupied(entry) => {
            let remove: bool = if let SegmentValue::End(_) = entry.get() { true } else { false };
            if remove {
                if let SegmentValue::End(v) = entry.remove() { v } else { panic!("We just checked this is End") }
            } else {
                let entry = entry.into_mut();
                let mut owned = SegmentValue::Start;
                mem::swap(entry, &mut owned);
                match owned {
                    SegmentValue::Start => panic!("Expected End/StartEnd"),
                    SegmentValue::End(_) => panic!("We just checked this isn't End"),
                    SegmentValue::StartEnd(v) => v,
                }
            }
        },
    }
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
                SegmentValue::Start | SegmentValue::StartEnd(_) => panic!("Didn't expect Start/StartEnd"),
                SegmentValue::End(v) => v,
            };
            let mut owned = SegmentValue::StartEnd(v);
            mem::swap(entry, &mut owned);
        },
    }
}

fn add_end<'a, V>(entry: InnerEntry<'a, V>, v: V) -> &'a mut V {
    match entry {
        BEntry::Vacant(entry) => entry.insert(SegmentValue::End(v)),
        BEntry::Occupied(entry) => {
            let entry = entry.into_mut();
            let mut owned = SegmentValue::StartEnd(v);
            mem::swap(entry, &mut owned);
            match owned {
                SegmentValue::End(_) | SegmentValue::StartEnd(_) => panic!("Didn't expect End/StartEnd"),
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
    seg: Segment,
}

#[derive(Debug)]
pub struct OccupiedEntry<'a, V> {
    tree: &'a mut SegmentTree<V>,
    seg: Segment,
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

    pub fn key(&self) -> &Segment {
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
    pub fn key(&self) -> &Segment {
        &self.seg
    }

    pub fn into_key(self) -> Segment {
        self.seg
    }

    pub fn insert(self, value: V) -> &'a mut V {
        add_start(self.tree.0.entry(self.seg.start));
        add_end(self.tree.0.entry(self.seg.end), value)
    }
}

impl<'a, V> OccupiedEntry<'a, V> {
    pub fn key(&self) -> &Segment {
        &self.seg
    }

    pub fn remove_entry(self) -> (Segment, V) {
        (self.seg, self.remove())
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

    pub fn remove(self) -> V {
        remove_start(self.tree.0.entry(self.seg.start));
        remove_end(self.tree.0.entry(self.seg.end))
    }
}

macro_rules! impl_segment {
    { $iter:expr, $seg:expr, $missing:expr, $found:expr } => {
        let mut iter = $iter;
        match iter.next() {
            None => $missing,
            Some((start, SegmentValue::Start)) |
            Some((start, SegmentValue::StartEnd(_))) if start == &$seg.start => {
                match iter.next() {
                    Some((end, SegmentValue::End(v))) |
                    Some((end, SegmentValue::StartEnd(v))) if end == &$seg.end => {
                        match iter.next() {
                            None => $found(v),
                            _ => Err(SegmentTreeError::Intersect),
                        }
                    }
                    _ => Err(SegmentTreeError::Intersect),
                }
            },
            Some((start, SegmentValue::End(_))) if start == &$seg.start => {
                match iter.next() {
                    None => $missing,
                    Some((end, SegmentValue::Start)) if end == &$seg.end => {
                        match iter.next() {
                            None => $missing,
                            _ => Err(SegmentTreeError::Intersect),
                        }
                    }
                    _ => Err(SegmentTreeError::Intersect),
                }
            }
            Some((end, SegmentValue::Start)) if end == &$seg.end => {
                match iter.next() {
                    None => $missing,
                    _ => Err(SegmentTreeError::Intersect),
                }
            }
            Some(_) => Err(SegmentTreeError::Intersect),
        }
    }
}

impl<T> SegmentTree<T> {
    pub fn new() -> Self { SegmentTree(BTreeMap::new()) }

    pub fn get_segment(&self, seg: Segment) -> Result<Option<&T>> {
        impl_segment! { self.0.range(seg.get_range()), seg, Ok(None), |v| Ok(Some(v)) }
    }

    pub fn get_mut_segment(&mut self, seg: Segment) -> Result<Option<&mut T>> {
        impl_segment! { self.0.range_mut(seg.get_range()), seg, Ok(None), |v| Ok(Some(v)) }
    }

    pub fn contains_segment(&self, seg: Segment) -> Result<bool> {
        impl_segment! { self.0.range(seg.get_range()), seg, Ok(false), |_| Ok(true) }
    }

    /// Gets an Ok(Entry), Vacant or Occupied, if the tree doesn't contain any intersection with the
    /// segment or contains it exactly. Returns None otherwise.
    pub fn entry_segment(&mut self, seg: Segment) -> Result<Entry<T>> {
        Ok(if self.contains_segment(seg)? {
            Entry::Occupied(OccupiedEntry { tree: self, seg })
        } else {
            Entry::Vacant(VacantEntry { tree: self, seg })
        })
    }

    pub fn insert_segment(&mut self, seg: Segment, value: T) -> std::result::Result<Option<T>, (T, SegmentTreeError)> {
        match self.entry_segment(seg) {
            Ok(Entry::Vacant(entry)) => {
                entry.insert(value);
                Ok(None)
            },
            Ok(Entry::Occupied(mut entry)) => Ok(Some(entry.insert(value))),
            Err(e) => Err((value, e)),
        }
    }

    pub fn remove_segment(&mut self, seg: Segment) -> Result<Option<T>> {
        Ok(match self.entry_segment(seg)? {
            Entry::Vacant(_) => None,
            Entry::Occupied(entry) => Some(entry.remove()),
        })
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
        assert_eq!(st.get_segment(Segment::new(1, 3)), Ok(None));
        assert_eq!(st.get_mut_segment(Segment::new(1, 3)), Ok(None));
        assert_eq!(st.contains_segment(Segment::new(1, 3)), Ok(false));
        assert_eq!(st.insert_segment(Segment::new(1, 3), X(0)), Ok(None));
        assert_eq!(st.get_segment(Segment::new(1, 3)), Ok(Some(&X(0))));
        assert_eq!(st.get_mut_segment(Segment::new(1, 3)), Ok(Some(&mut X(0))));
        assert_eq!(st.contains_segment(Segment::new(1, 3)), Ok(true));
        assert_eq!(st.contains_segment(Segment::new(2, 3)), Err(SegmentTreeError::Intersect));
        assert_eq!(st.contains_segment(Segment::new(2, 4)), Err(SegmentTreeError::Intersect));
        assert_eq!(st.contains_segment(Segment::new(0, 2)), Err(SegmentTreeError::Intersect));
        assert_eq!(st.contains_segment(Segment::new(0, 4)), Err(SegmentTreeError::Intersect));
        assert_eq!(st.contains_segment(Segment::new(3, 6)), Ok(false));
        assert_eq!(st.contains_segment(Segment::new(0, 1)), Ok(false));
        assert_eq!(st.insert_segment(Segment::new(7, 9), X(1)), Ok(None));
        assert_eq!(st.insert_segment(Segment::new(1, 5), X(2)), Err((X(2), SegmentTreeError::Intersect)));
        assert_eq!(st.insert_segment(Segment::new(1, 3), X(3)), Ok(Some(X(0))));
        assert_eq!(st.insert_segment(Segment::new(3, 4), X(4)), Ok(None));
        assert_eq!(st.contains_segment(Segment::new(4, 7)), Ok(false));
        assert_let!(Ok(Entry::Vacant(entry)) = st.entry_segment(Segment::new(5, 7)), {
            assert_eq!(entry.insert(X(5)), &mut X(5));
        });
        assert_eq!(st.insert_segment(Segment::new(4, 5), X(6)), Ok(None));
        assert_eq!(st.get_segment(Segment::new(1, 3)), Ok(Some(&X(3))));
        assert_eq!(st.get_segment(Segment::new(3, 4)), Ok(Some(&X(4))));
        assert_eq!(st.get_segment(Segment::new(4, 5)), Ok(Some(&X(6))));
        assert_eq!(st.get_segment(Segment::new(5, 7)), Ok(Some(&X(5))));
        assert_let!(Ok(Entry::Occupied(entry)) = st.entry_segment(Segment::new(4, 5)), {
            assert_eq!(entry.remove(), X(6));
        });
        assert_eq!(st.get_segment(Segment::new(4, 5)), Ok(None));
        assert_let!(Ok(Entry::Occupied(mut entry)) = st.entry_segment(Segment::new(5, 7)), {
            assert_eq!(entry.insert(X(7)), X(5));
        });
        assert_let!(Err(SegmentTreeError::Intersect) = st.entry_segment(Segment::new(0, 9)));
        assert_eq!(st.remove_segment(Segment::new(0, 9)), Err(SegmentTreeError::Intersect));
        assert_eq!(st.remove_segment(Segment::new(4, 5)), Ok(None));
        assert_eq!(st.remove_segment(Segment::new(5, 7)), Ok(Some(X(7))));
    }
}
