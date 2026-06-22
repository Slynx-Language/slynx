use std::{hash::Hash, ops::Deref};

use crate::pool::{Pool, PoolId};

pub struct PoolViewer<'a, T: Hash> {
    pub(crate) pool: &'a Pool<T>,
    pub(crate) id: PoolId<T>,
}

impl<'a, T> Deref for PoolViewer<'a, T>
where
    T: Hash + Clone + Eq,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.pool.get(self.id)
    }
}
