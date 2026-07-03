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
        )*
        $crate::paste!{
            impl $name {
                $(
                    pub fn [<insert_at_ $field_name>](&self, value: $ty) -> DedupPoolId<$ty>{
                        self.$field_name.insert(value)
                    }
                )*
            }
        }

    };
}
#[macro_export]
macro_rules! pooled {
    ($v:vis $name: ident {
        $($fvis:vis $field_name:ident : $ty:ty),* $(,)?
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
            impl std::ops::IndexMut<PoolId<$ty>> for $name {
                fn index_mut(&mut self, index: PoolId<$ty>) -> &mut Self::Output {
                    self.$field_name.get_mut(index)
                }
            }
        )*
        $crate::paste!{
            impl $name {
                $(
                    pub fn [<insert_at_ $field_name>](&self, value: $ty) -> PoolId<$ty>{
                        self.$field_name.insert(value)
                    }
                )*
            }
        }

    };
}
