use crate::{
    AliasDeclaration, ComponentDeclaration, EnumDeclaration, FileImport, FuncDeclaration,
    ObjectDeclaration, StaticDeclaration, StyleSheet,
};
use common::pool::Pool;
use paste::paste;

macro_rules! program {
    ($($name:ident : $typ:ty),+ $(,)?) => {
        #[derive(Debug)]
        pub struct Program {
            $(
                $name: Pool<$typ>,
            )*
        }
        impl Default for Program {
            fn default() -> Self {
                Self::new()
            }
        }
        impl Program {
            pub fn new() -> Self {
                Self {
                    $($name: Pool::new(),)*
                }
            }
            $(pub fn $name(&self) -> &Pool<$typ> {
                &self.$name
            })*
            $(paste!{
                pub fn [<append_ $name>](&self, data: $typ) {
                    self.$name.insert(data);
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
    statics: StaticDeclaration,
    enums: EnumDeclaration
}
