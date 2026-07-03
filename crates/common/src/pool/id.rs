use std::{hash::Hash, marker::PhantomData, u32};

#[derive(Debug)]
pub struct DedupPoolId<T>(pub(crate) u32, pub(crate) PhantomData<T>);
#[derive(Debug)]
pub struct PoolId<T>(pub(crate) u32, pub(crate) PhantomData<T>);

macro_rules! impl_derives {
    ($($ty:ident),*$(,)?) => {
        $(
            impl<T> Hash for $ty<T> {
                fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                    self.0.hash(state);
                }
            }

            impl<T> PartialEq for $ty<T> {
                fn eq(&self, other: &Self) -> bool {
                    self.0 == other.0
                }
            }
            impl<T> Eq for $ty<T> {}

            impl<T> Clone for $ty<T> {
                fn clone(&self) -> Self {
                    Self(self.0, self.1)
                }
            }

            impl<T> Copy for $ty<T> {}
            impl<T> $ty<T> {
                pub fn new(inner: u32) -> Self {
                    Self(inner, PhantomData)
                }
                pub fn new_null() -> Self {
                    Self(u32::MAX, PhantomData)
                }

                pub fn as_raw(&self) -> u32 {
                    self.0
                }
                pub fn is_null(&self) -> bool {
                    self.0 == u32::MAX
                }
            }
        )*
    };
}

impl_derives!(DedupPoolId, PoolId);
