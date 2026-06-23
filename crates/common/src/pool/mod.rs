mod view;
use std::{hash::Hash, marker::PhantomData, ops::Index};
pub use view::*;

use dashmap::DashMap;
#[derive(Debug, Hash)]
pub struct PoolId<T>(u32, PhantomData<T>);

impl<T> PartialEq for PoolId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for PoolId<T> {}

impl<T> Clone for PoolId<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}
impl<T> Copy for PoolId<T> {}
impl<T> PoolId<T> {
    ///THis method is unsafe because it does not guarantee the returned pool id will be avaible on a pool
    pub unsafe fn from_raw(raw: u32) -> Self {
        Self(raw, PhantomData)
    }
    pub fn inner(&self) -> u32 {
        self.0
    }
}

pub struct Pool<T: Hash> {
    pub(crate) inner: boxcar::Vec<T>,
    pub(crate) hashes: DashMap<T, PoolId<T>>,
}
impl<T: Hash + Eq + Clone> Pool<T> {
    pub fn new() -> Self {
        Self {
            inner: boxcar::Vec::new(),
            hashes: DashMap::new(),
        }
    }
    ///Inserts the given `data` into this pool. If it was previously inserted returns the ID of the previous value
    pub fn insert(&self, data: T) -> PoolId<T> {
        if let Some(data) = self.hashes.get(&data) {
            *data
        } else {
            let index = self.inner.push(data.clone());
            let out = PoolId(index as u32, PhantomData);
            self.hashes.insert(data, out);
            out
        }
    }

    ///Gets the data that originated the given `id`
    pub fn get(&self, id: PoolId<T>) -> &T {
        self.inner
            .get(id.0 as usize)
            .expect("Expected to retrieve data from pool id originated from insert")
    }

    pub fn view<'a>(&'a self, id: PoolId<T>) -> PoolViewer<'a, T> {
        PoolViewer { pool: self, id: id }
    }
}

impl<T> Index<PoolId<T>> for Pool<T>
where
    T: Hash + Eq + Clone,
{
    type Output = T;
    fn index(&self, index: PoolId<T>) -> &Self::Output {
        self.get(index)
    }
}
