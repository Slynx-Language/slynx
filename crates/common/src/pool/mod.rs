mod id;
pub mod soa;
use std::fmt::Debug;
use std::{hash::Hash, marker::PhantomData, ops::Index};

use dashmap::DashMap;

pub use crate::pool::id::DedupPoolId;
pub use crate::pool::id::PoolId;

#[derive(Default)]
pub struct DedupPool<T: Eq + Hash> {
    pub(crate) inner: boxcar::Vec<T>,
    pub(crate) hashes: DashMap<T, DedupPoolId<T>>,
}

impl<T> Debug for DedupPool<T>
where
    T: Debug + Eq + Hash,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

#[derive(Debug, Default)]
pub struct Pool<T> {
    pub(crate) inner: boxcar::Vec<T>,
}
impl<T: Hash + Eq + Clone> DedupPool<T> {
    pub fn new() -> Self {
        Self {
            inner: boxcar::Vec::new(),
            hashes: DashMap::new(),
        }
    }
    ///Inserts the given `data` into this pool. If it was previously inserted returns the ID of the previous value
    pub fn insert(&self, data: T) -> DedupPoolId<T> {
        *self
            .hashes
            .entry(data.clone())
            .or_insert_with(|| {
                let index = self.inner.push(data);
                DedupPoolId(index as u32, PhantomData)
            })
            .value()
    }

    ///Gets the data that originated the given `id`
    pub fn get(&self, id: DedupPoolId<T>) -> &T {
        self.inner.get(id.as_raw() as usize).unwrap_or_else(|| {
            panic!(
                "Expected to retrieve data from pool id originated from insert (id={} count={})",
                id.as_raw(),
                self.inner.count()
            )
        })
    }

    pub fn len(&self) -> usize {
        self.inner.count()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    ///Gets a mutable reference to the data that originated the given `id`.
    ///
    /// Note: mutating a value that was inserted into a dedup pool invalidates
    /// its stored hash key. Only mutate values whose hash is stable under the
    /// mutation (e.g. structs dedup'd by name) and never re-`insert` a value
    /// that expects to be dedup'd on the mutated fields.
    pub fn get_mut(&mut self, id: DedupPoolId<T>) -> &mut T {
        self.inner
            .get_mut(id.as_raw() as usize)
            .expect("Expected to retrieve data from pool id originated from insert")
    }

    ///Iterates over all values stored on this pool, yielding their `id` and a reference to the data
    pub fn iter(&self) -> impl Iterator<Item = (DedupPoolId<T>, &T)> {
        self.inner
            .iter()
            .map(|(index, value)| (DedupPoolId::new(index as u32), value))
    }
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Self {
            inner: boxcar::Vec::new(),
        }
    }
    ///Inserts the given `data` into this pool. If it was previously inserted returns the ID of the previous value
    pub fn insert(&self, data: T) -> PoolId<T> {
        let idx = self.inner.push(data);
        PoolId(idx as u32, PhantomData)
    }

    ///Gets the data that originated the given `id`
    pub fn get(&self, id: PoolId<T>) -> &T {
        self.inner
            .get(id.as_raw() as usize)
            .expect("Expected to retrieve data from pool id originated from insert")
    }
    pub fn iter<'a>(&'a self) -> PoolIterator<'a, T> {
        PoolIterator {
            pool: self,
            current: 0,
        }
    }
    ///Gets the data that originated the given `id`
    pub fn get_mut(&mut self, id: PoolId<T>) -> &mut T {
        self.inner
            .get_mut(id.as_raw() as usize)
            .expect("Expected to retrieve data from pool id originated from insert")
    }
}
impl<T> Index<PoolId<T>> for Pool<T> {
    type Output = T;
    fn index(&self, index: PoolId<T>) -> &Self::Output {
        self.get(index)
    }
}
impl<T> Index<DedupPoolId<T>> for DedupPool<T>
where
    T: Hash + Eq + Clone,
{
    type Output = T;
    fn index(&self, index: DedupPoolId<T>) -> &Self::Output {
        self.get(index)
    }
}

pub struct PoolIterator<'a, T> {
    pool: &'a Pool<T>,
    current: usize,
}

impl<'a, T> Iterator for PoolIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        let out = self.pool.inner.get(self.current);
        self.current += 1;
        out
    }
}

pub struct IndexedPoolIterator<'a, T> {
    pool: &'a Pool<T>,
    current: usize,
}

impl<'a, T> Iterator for IndexedPoolIterator<'a, T> {
    type Item = (PoolId<T>, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        let out = self
            .pool
            .inner
            .get(self.current)
            .map(|out| (PoolId::new(self.current as u32), out));
        self.current += 1;
        out
    }
}

impl<'a, T> PoolIterator<'a, T> {
    pub fn with_ids(self) -> IndexedPoolIterator<'a, T> {
        IndexedPoolIterator {
            pool: self.pool,
            current: self.current,
        }
    }
}
