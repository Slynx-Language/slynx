mod view;
use std::{hash::Hash, marker::PhantomData};
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

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct RefPoolId<'a, T>(u32, PhantomData<&'a T>);

impl<'a, T> Clone for RefPoolId<'a, T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}
impl<'a, T> Copy for RefPoolId<'a, T> {}

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
    ///Inserts the given `data` into this pool. This differences between `insert` because the reference is tied to the lifetime of this pool
    ///and so another pool might not be able to generate data and use it wrongly
    pub fn insert_lifetime(&self, data: T) -> RefPoolId<'_, T> {
        let data = self.insert(data);
        RefPoolId(data.0, PhantomData)
    }
    ///Gets the data that originated the given `id`
    pub fn get(&self, id: PoolId<T>) -> &T {
        self.inner
            .get(id.0 as usize)
            .expect("Expected to retrieve data from pool id originated from insert")
    }
    ///Gets the data that originated the given `id`
    pub fn get_lifetime(&self, id: RefPoolId<'_, T>) -> &T {
        self.get(PoolId(id.0, PhantomData))
    }

    pub fn view<'a>(&'a self, id: PoolId<T>) -> PoolViewer<'a, T> {
        PoolViewer { pool: self, id: id }
    }
}
