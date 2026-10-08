mod id;
pub mod soa;
use std::fmt::Debug;
use std::{hash::Hash, marker::PhantomData, ops::Index};

use dashmap::DashMap;

pub use crate::pool::id::DedupPoolId;
pub use crate::pool::id::PoolId;
use crate::vec::{AppendOnlyVec, TryLike};

#[derive(Default)]
pub struct DedupPool<T: Eq + Hash> {
    pub(crate) inner: AppendOnlyVec<T>,
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
    pub(crate) inner: AppendOnlyVec<T>,
}
impl<T: Hash + Eq + Clone> DedupPool<T> {
    pub fn new() -> Self {
        Self {
            inner: AppendOnlyVec::new(),
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
        unsafe { self.inner.get_unchecked(id.as_raw() as usize) }
    }
    pub fn get_mut(&mut self, id: DedupPoolId<T>) -> &mut T {
        unsafe { self.inner.get_unchecked_mut(id.as_raw() as usize) }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    ///Iterates over all values stored on this pool, yielding their `id` and a reference to the data
    pub fn iter(&self) -> impl Iterator<Item = (DedupPoolId<T>, &T)> {
        self.inner
            .iter()
            .enumerate()
            .map(|(index, value)| (DedupPoolId::new(index as u32), value))
    }
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Self {
            inner: AppendOnlyVec::new(),
        }
    }
    pub fn push_with_next_id<R>(&self, f: impl FnOnce(usize) -> R) -> <R as TryLike>::Rebuilt<usize>
    where
        R: TryLike<Output = T>,
    {
        self.inner.push_with_next_id(f)
    }
    pub fn insert_with_id<R: TryLike<Output = T>>(
        &self,
        f: impl FnOnce(PoolId<T>) -> R,
    ) -> <R as TryLike>::Rebuilt<PoolId<T>> {
        let s = self.inner.push_with_next_id(|id| {
            let id = PoolId::new(id as u32);
            f(id)
        });
        R::map_rebuilt(s, |s| PoolId::new(s as u32))
    }

    ///Inserts the given `data` into this pool. If it was previously inserted returns the ID of the previous value
    pub fn insert(&self, data: T) -> PoolId<T> {
        let idx = self.inner.push(data);
        PoolId(idx as u32, PhantomData)
    }

    ///Gets the data that originated the given `id`
    pub fn get(&self, id: PoolId<T>) -> &T {
        unsafe { self.inner.get_unchecked(id.as_raw() as usize) }
    }
    pub fn get_mut(&mut self, id: PoolId<T>) -> &mut T {
        unsafe { self.inner.get_unchecked_mut(id.as_raw() as usize) }
    }
    pub fn iter<'a>(&'a self) -> PoolIterator<'a, T> {
        PoolIterator {
            pool: self,
            current: 0,
        }
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
        if self.current >= self.pool.inner.len() {
            return None;
        }
        let out = self.pool.get(PoolId::new(self.current as u32));
        self.current += 1;
        Some(out)
    }
}

pub struct IndexedPoolIterator<'a, T> {
    pool: &'a Pool<T>,
    current: usize,
}

impl<'a, T> Iterator for IndexedPoolIterator<'a, T> {
    type Item = (PoolId<T>, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.pool.inner.len() {
            return None;
        }
        let id = PoolId::new(self.current as u32);
        let out = self.pool.get(id);
        self.current += 1;
        Some((id, out))
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
