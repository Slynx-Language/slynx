use crate::{
    AliasDeclaration, ComponentDeclaration, FileImport, FuncDeclaration, ObjectDeclaration,
    StaticDeclaration, StyleSheet,
};
use paste::paste;
use std::marker::PhantomData;

macro_rules! program {
    ($($name:ident : $typ:ty),+ $(,)?) => {
        #[derive(Debug)]
        pub struct Program<'a> {
            phantom: PhantomData<&'a ()>,
            $(
                $name: Vec<$typ>,
            )*
        }
        impl<'a> Program<'a> {
            pub fn new() -> Self {
                Self {
                    phantom: PhantomData,
                    $($name: Vec::new(),)*
                }
            }
            $(pub fn $name(&self) -> &[$typ] {
                &self.$name
            })*
            $(paste!{
                pub fn [<append_ $name>](&mut self, data: $typ) {
                    self.$name.push(data);
                }
            })*
        }
    };
}
program! {
    imports: FileImport,
    alias: AliasDeclaration,
    object: ObjectDeclaration,
    component: ComponentDeclaration,
    func: FuncDeclaration,
    style: StyleSheet,
    statics: StaticDeclaration
}
