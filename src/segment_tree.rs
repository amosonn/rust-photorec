use std::collections::btree_map::Entry as BEntry;
use std::collections::BTreeMap;
use std::mem;
use core::ops::RangeInclusive;

use Entry::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct Segment {
    pub start: u64,
    pub end: u64,
}

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

#[derive(PartialEq, Eq, Debug)]
pub enum Get<'a, T> {
    /// The exact segment is in the tree, with this value
    Exact(&'a T),
    /// This segment intersects one or more segments in the tree
    Intersect,
    /// This segment doesn't intersect any segment in the tree
    Doesnt,
}

#[derive(PartialEq, Eq, Debug)]
pub enum GetMut<'a, T> {
    /// The exact segment is in the tree, with this value
    Exact(&'a mut T),
    /// This segment intersects one or more segments in the tree
    Intersect,
    /// This segment doesn't intersect any segment in the tree
    Doesnt,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Insert<T> {
    /// The exact segment is in the tree, this was the old value
    Old(T),
    /// This segment intersects one or more segments in the tree, the argument is returned
    Intersect(T),
    /// Inserted successfully
    Inserted,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Contains {
    /// The exact segment is in the tree
    Exact,
    /// This segment intersects one or more segments in the tree
    Intersect,
    /// This segment doesn't intersect any segment in the tree
    Doesnt,
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

impl<'a, T> From<Get<'a, T>> for Contains {
    fn from(g: Get<'a, T>) -> Self {
        match g {
            Get::Exact(_) => Contains::Exact,
            Get::Intersect => Contains::Intersect,
            Get::Doesnt => Contains::Doesnt,
        }
    }
}

impl<T> SegmentTree<T> {
    pub fn new() -> Self { SegmentTree(BTreeMap::new()) }

    pub fn get_segment(&self, seg: Segment) -> Get<T> {
        let mut iter = self.0.range(seg.get_range());
        match iter.next() {
            None => Get::Doesnt,
            Some((start, SegmentValue::Start)) |
            Some((start, SegmentValue::StartEnd(_))) if start == &seg.start => {
                match iter.next() {
                    Some((end, SegmentValue::End(v))) |
                    Some((end, SegmentValue::StartEnd(v))) if end == &seg.end => {
                        match iter.next() {
                            None => Get::Exact(v),
                            _ => Get::Intersect,
                        }
                    }
                    _ => Get::Intersect,
                }
            },
            Some((start, SegmentValue::End(_))) if start == &seg.start => {
                match iter.next() {
                    None => Get::Doesnt,
                    Some((end, SegmentValue::Start)) if end == &seg.end => {
                        match iter.next() {
                            None => Get::Doesnt,
                            _ => Get::Intersect,
                        }
                    }
                    _ => Get::Intersect,
                }
            }
            Some((end, SegmentValue::Start)) if end == &seg.end => {
                match iter.next() {
                    None => Get::Doesnt,
                    _ => Get::Intersect,
                }
            }
            Some(_) => Get::Intersect,
        }
    }

    pub fn get_mut_segment(&mut self, seg: Segment) -> GetMut<T> {
        let mut iter = self.0.range_mut(seg.get_range());
        match iter.next() {
            None => GetMut::Doesnt,
            Some((start, SegmentValue::Start)) |
            Some((start, SegmentValue::StartEnd(_))) if start == &seg.start => {
                match iter.next() {
                    Some((end, SegmentValue::End(v))) |
                    Some((end, SegmentValue::StartEnd(v))) if end == &seg.end => {
                        match iter.next() {
                            None => GetMut::Exact(v),
                            _ => GetMut::Intersect,
                        }
                    }
                    _ => GetMut::Intersect,
                }
            },
            Some((start, SegmentValue::End(_))) if start == &seg.start => {
                match iter.next() {
                    None => GetMut::Doesnt,
                    Some((end, SegmentValue::Start)) if end == &seg.end => {
                        match iter.next() {
                            None => GetMut::Doesnt,
                            _ => GetMut::Intersect,
                        }
                    }
                    _ => GetMut::Intersect,
                }
            }
            Some((end, SegmentValue::Start)) if end == &seg.end => {
                match iter.next() {
                    None => GetMut::Doesnt,
                    _ => GetMut::Intersect,
                }
            }
            Some(_) => GetMut::Intersect,
        }
    }

    pub fn contains_segment(&self, seg: Segment) -> Contains { self.get_segment(seg).into() }

    /// Gets an Ok(Entry), Vacant or Occupied, if the tree doesn't contain any intersection with the
    /// segment or contains it exactly. Returns None otherwise.
    pub fn entry_segment(&mut self, seg: Segment) -> Option<Entry<T>> {
        match self.contains_segment(seg) {
            Contains::Exact => Some(Entry::Occupied(OccupiedEntry { tree: self, seg })),
            Contains::Intersect => None,
            Contains::Doesnt => Some(Entry::Vacant(VacantEntry { tree: self, seg })),
        }
    }

    pub fn insert_segment(&mut self, seg: Segment, value: T) -> Insert<T> {
        match self.entry_segment(seg) {
            Some(Entry::Vacant(entry)) => {
                entry.insert(value);
                Insert::Inserted
            },
            Some(Entry::Occupied(mut entry)) => Insert::Old(entry.insert(value)),
            None => Insert::Intersect(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Segment, SegmentTree, Get, GetMut, Insert, Contains, Entry};
    #[test]
    fn smoke() {
        #[derive(Debug, PartialEq, Eq)]
        struct X(u64);

        let mut st = SegmentTree::new();
        assert_eq!(st.get_segment(Segment::new(1, 3)), Get::Doesnt);
        assert_eq!(st.get_mut_segment(Segment::new(1, 3)), GetMut::Doesnt);
        assert_eq!(st.contains_segment(Segment::new(1, 3)), Contains::Doesnt);
        assert_eq!(st.insert_segment(Segment::new(1, 3), X(0)), Insert::Inserted);
        assert_eq!(st.get_segment(Segment::new(1, 3)), Get::Exact(&X(0)));
        assert_eq!(st.get_mut_segment(Segment::new(1, 3)), GetMut::Exact(&mut X(0)));
        assert_eq!(st.contains_segment(Segment::new(1, 3)), Contains::Exact);
        assert_eq!(st.contains_segment(Segment::new(2, 3)), Contains::Intersect);
        assert_eq!(st.contains_segment(Segment::new(2, 4)), Contains::Intersect);
        assert_eq!(st.contains_segment(Segment::new(0, 2)), Contains::Intersect);
        assert_eq!(st.contains_segment(Segment::new(0, 4)), Contains::Intersect);
        assert_eq!(st.contains_segment(Segment::new(3, 6)), Contains::Doesnt);
        assert_eq!(st.contains_segment(Segment::new(0, 1)), Contains::Doesnt);
        assert_eq!(st.insert_segment(Segment::new(7, 9), X(1)), Insert::Inserted);
        assert_eq!(st.insert_segment(Segment::new(1, 5), X(2)), Insert::Intersect(X(2)));
        assert_eq!(st.insert_segment(Segment::new(1, 3), X(3)), Insert::Old(X(0)));
        assert_eq!(st.insert_segment(Segment::new(3, 4), X(4)), Insert::Inserted);
        assert_eq!(st.contains_segment(Segment::new(4, 7)), Contains::Doesnt);
        assert_let!(Some(Entry::Vacant(entry)) = st.entry_segment(Segment::new(5, 7)), {
            assert_eq!(entry.insert(X(5)), &mut X(5));
        });
        assert_eq!(st.insert_segment(Segment::new(4, 5), X(6)), Insert::Inserted);
        assert_eq!(st.get_segment(Segment::new(1, 3)), Get::Exact(&X(3)));
        assert_eq!(st.get_segment(Segment::new(3, 4)), Get::Exact(&X(4)));
        assert_eq!(st.get_segment(Segment::new(4, 5)), Get::Exact(&X(6)));
        assert_eq!(st.get_segment(Segment::new(5, 7)), Get::Exact(&X(5)));
        assert_let!(Some(Entry::Occupied(entry)) = st.entry_segment(Segment::new(4, 5)), {
            assert_eq!(entry.remove(), X(6));
        });
        assert_eq!(st.get_segment(Segment::new(4, 5)), Get::Doesnt);
        assert_let!(Some(Entry::Occupied(mut entry)) = st.entry_segment(Segment::new(5, 7)), {
            assert_eq!(entry.insert(X(7)), X(5));
        });
        assert_let!(None = st.entry_segment(Segment::new(0, 9)));
    }
}
