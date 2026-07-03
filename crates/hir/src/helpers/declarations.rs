use module_loader::FileId;

use crate::HirFile;
use crate::{
    DeclarationId, HirAliasDeclaration, HirComponentDeclaration, HirFunctionDeclaration,
    HirObjectDeclaration, HirStaticDeclaration, HirStylesheetDeclaration, SlynxHir,
};
use dashmap::mapref::one::MappedRef;

macro_rules! get_data {
    ($($name:ident($typ: ty)),*$(,)?) => {
        impl SlynxHir<'_>{
            paste::paste! {
                $(pub(crate) fn [<get_ $name>](&self, id: DeclarationId<$typ>) -> MappedRef<'_, FileId, HirFile, $typ> {
                    self.get_file(id.file_id).map(|file| &file[id.local_id])
                })*
            }
        }
    };
}

get_data!(
    function(HirFunctionDeclaration),
    alias(HirAliasDeclaration),
    static(HirStaticDeclaration),
    object(HirObjectDeclaration),
    component(HirComponentDeclaration),
    style(HirStylesheetDeclaration),
);
