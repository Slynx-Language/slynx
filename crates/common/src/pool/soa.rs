use std::hash::Hash;

use crate::pool::{DedupPool, Pool};

pub trait PoolStorage<T> {
    fn get_pool(&self) -> &Pool<T>;
}
pub trait DedupPoolStorage<T: Hash + Eq> {
    fn get_pool(&self) -> &DedupPool<T>;
}

#[macro_export]
macro_rules! dedup_pooled {
    ($v:vis $name: ident {
        $($fvis:vis $field_name:ident : $ty:ty),* $(,)?
    }) => {
        $v struct $name {
            $($fvis $field_name: DedupPool<$ty>,)*
        }
        impl std::default::Default for $name {
            fn default() -> Self {
                Self {
                    $($field_name: DedupPool::new()),*
                }
            }
        }

        $(
            impl std::ops::Index<DedupPoolId<$ty>> for $name {
                type Output = $ty;
                fn index(&self, index: DedupPoolId<$ty>) -> &Self::Output {
                    self.$field_name.get(index)
                }
            }
            impl DedupPoolStorage<$ty> for $name {
                fn get_pool(&self) -> &DedupPool<$ty> {
                    &self.$field_name
                }
            }
        )*
        impl $name {
            pub fn insert<T>(&self, data: T) -> DedupPoolId<T> where Self:DedupPoolStorage<T>, T:std::hash::Hash+std::cmp::Eq+Clone{
                self.get_pool().insert(data)
            }
        }
        $crate::paste!{
            impl $name {
                $(
                    pub fn [<insert_at_ $field_name>](&self, value: $ty) -> DedupPoolId<$ty>{
                        self.$field_name.insert(value)
                    }
                    pub fn $field_name(&self) -> &DedupPool<$ty> {
                        &self.$field_name
                    }
                )*

            }
        }

    };
}
#[macro_export]
#[doc(hidden)]
macro_rules! __pooled_insert_with_id {
    // sem errwith: closure infalível
    ($field:ident, $ty:ty) => {
        $crate::paste! {
            pub fn [<insert_at_ $field _with_id>]<F: FnOnce(PoolId<$ty>) -> $ty>(
                &self,
                f: F,
            ) -> PoolId<$ty> {
                self.$field.insert_with_id(f)
            }
        }
    };
    // com errwith Result: closure falível
    ($field:ident, $ty:ty, $err:ident) => {
        $crate::paste! {
            pub fn [<insert_at_ $field _with_id>]<F: FnOnce(PoolId<$ty>) -> $err<$ty>>(
                &self,
                f: F,
            ) -> $err<PoolId<$ty>> {
                self.$field.insert_with_id(f)
            }
        }
    };
}
#[macro_export]
macro_rules! pooled {
    ($v:vis $name: ident {
        $($fvis:vis $field_name:ident : $ty:ty $(where Err=$err:ident)?),* $(,)?
    }) => {
        $v struct $name {
            $($fvis $field_name: Pool<$ty>,)*
        }
        impl std::default::Default for $name {
            fn default() -> Self {
                Self {
                    $($field_name: Pool::new()),*
                }
            }
        }

        $(
            impl std::ops::Index<PoolId<$ty>> for $name {
                type Output = $ty;
                fn index(&self, index: PoolId<$ty>) -> &Self::Output {
                    self.$field_name.get(index)
                }
            }
            impl PoolStorage<$ty> for $name {
                fn get_pool(&self) -> &Pool<$ty> {
                    &self.$field_name
                }
            }

        )*
        impl $name {
            pub fn insert<T>(&self, data: T) -> PoolId<T> where Self:PoolStorage<T> {
                self.get_pool().insert(data)
            }
        }
        impl $name {
            $(
                $crate::paste! {
                    pub fn [<insert_at_ $field_name>](&self, value: $ty) -> PoolId<$ty> {
                        self.$field_name.insert(value)
                    }
                    pub fn $field_name(&self) -> &Pool<$ty> {
                        &self.$field_name
                    }
                }
                $crate::__pooled_insert_with_id!($field_name, $ty $(, $err)?);
            )*
        }

    };
}
