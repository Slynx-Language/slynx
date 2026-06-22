use crate::{
    AliasDeclaration, ComponentDeclaration, FileImport, FuncDeclaration, ObjectDeclaration,
    StaticDeclaration, StyleSheet,
};
use paste::paste;

macro_rules! program {
    ($($name:ident : $typ:ty),+ $(,)?) => {
        #[derive(Debug)]
        pub struct Program {
            $(
                $name: Vec<$typ>,
            )*
        }
        impl Program {
            pub fn new() -> Self {
                Self {
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
