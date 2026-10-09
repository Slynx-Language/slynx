use crate::{
    AliasDeclaration, ComponentDeclaration, EnumDeclaration, ExtendDeclaration, FileImport,
    FuncDeclaration, InterfaceDeclaration, ObjectDeclaration, Result, StaticDeclaration,
    StyleSheet,
};
use common::PoolStorage;
use common::{
    pool::{Pool, PoolId},
    pooled,
};
pooled!(pub Program{
    imports: FileImport where Err=Result,
    alias: AliasDeclaration where Err=Result,
    object: ObjectDeclaration where Err=Result,
    component: ComponentDeclaration where Err=Result,
    func: FuncDeclaration where Err=Result,
    style: StyleSheet where Err=Result,
    statics: StaticDeclaration where Err=Result,
    enums: EnumDeclaration where Err=Result,
    interfaces: InterfaceDeclaration where Err=Result,
    extensions: ExtendDeclaration where Err=Result,
});

impl std::fmt::Debug for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Program")
            .field("imports", &self.imports)
            .field("alias", &self.alias)
            .field("object", &self.object)
            .field("component", &self.component)
            .field("func", &self.func)
            .field("style", &self.style)
            .field("statics", &self.statics)
            .field("enums", &self.enums)
            .field("interfaces", &self.interfaces)
            .field("extensions", &self.extensions)
            .finish()
    }
}
