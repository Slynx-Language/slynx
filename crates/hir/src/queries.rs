use common::Span;
use dashmap::mapref::one::{Ref, RefMut};
use module_loader::FileId;

use crate::{
    DeclarationId, DescriptorId, GenericParameter, HirFunctionDeclaration, Result, SlynxHir,
    SymbolPointer, VariableId,
    context::HirSymbol,
    helpers::HirViewer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
    term::{TermId, TermNode},
};

impl SlynxHir<'_> {
    /// Returns a method that the given `ty` owns with the given `name`. The method MUST be provided by an extension.
    pub fn get_extension_method(
        &self,
        ty: TermId,
        name: SymbolPointer,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        let extensions = self.types.methods.get_extensions_of(ty)?;
        for extensionid in extensions.iter() {
            let extension = self.get_extension(*extensionid);
            if extension.target != ty {
                continue;
            }
            if let Some(method) = extension
                .methods
                .iter()
                .find(|(method_name, _)| *method_name == name)
            {
                return Some(method.1);
            }
        }
        None
    }

    pub fn find_function_by_symbol(
        &self,
        symbol: HirSymbol,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.symbols_registry.get_function(symbol)
    }

    pub fn find_component_by_symbol(
        &self,
        symbol: HirSymbol,
    ) -> Option<DeclarationId<crate::HirComponentDeclaration>> {
        self.symbols_registry.get_component(symbol)
    }

    pub fn intern_name(&self, name: &str) -> SymbolPointer {
        self.symbols_resolver.intern(name)
    }

    pub fn get_name(&self, name: SymbolPointer) -> &str {
        self.symbols_resolver.get_name(name)
    }

    pub fn get_variable_name(&self, id: VariableId) -> &str {
        let ptr = self
            .store
            .variable_names
            .get(&id)
            .expect("Variable name should be registered during HIR construction");
        self.symbols_resolver.get_name(*ptr)
    }

    pub fn get_file(&self, id: FileId) -> Ref<'_, FileId, crate::file::HirFile> {
        self.store.get_file(id)
    }
    pub fn get_file_mut(&self, id: FileId) -> RefMut<'_, FileId, crate::file::HirFile> {
        self.store.get_file_mut(id)
    }

    pub fn get_declaration_type(&self, id: AnyDeclarationId) -> TermId {
        let file = self.store.get_or_create_file(id.owner);
        match id.term {
            AnyLocalDeclarationId::Alias(alias) => file.alias.get(alias).ty,
            AnyLocalDeclarationId::Component(component) => file.components.get(component).ty,
            AnyLocalDeclarationId::Function(func) => file.functions.get(func).ty,
            AnyLocalDeclarationId::Object(obj) => file.objects.get(obj).ty,
            AnyLocalDeclarationId::Static(statik) => file.statik.get(statik).ty,
            AnyLocalDeclarationId::Enum(enun) => file.enums.get(enun).ty,
        }
    }

    pub fn get_declaration_generics(&self, id: AnyDeclarationId) -> Vec<GenericParameter> {
        let file = self.store.get_or_create_file(id.owner);
        match id.term {
            AnyLocalDeclarationId::Function(func) => &file.functions.get(func).generics,
            AnyLocalDeclarationId::Alias(alias) => &file.alias.get(alias).generics,
            AnyLocalDeclarationId::Component(component) => &file.components.get(component).generics,
            AnyLocalDeclarationId::Object(obj) => &file.objects.get(obj).generics,
            AnyLocalDeclarationId::Enum(enun) => &file.enums.get(enun).generics,
            AnyLocalDeclarationId::Static(_) => {
                unreachable!("An static should not contain generics")
            }
        }
        .to_vec()
    }

    pub fn type_of_intrinsic(&self, name: SymbolPointer, span: Span) -> Result<TermId> {
        let id = self.store.lang_items.get(name, span)?;
        Ok(self.get_declaration_type(id))
    }

    /// Recursively flattens a HIR type to its primitive components.
    /// A struct `Color { inner: int }` flattens to `[int]`.
    /// A struct `Border { color: Color, width: int, radius: int }` flattens to `[int, int, int]`.
    pub fn flatten_type(&self, ty: TermId) -> Vec<TermId> {
        match &self.types[ty].node() {
            TermNode::Primitive(_) => vec![ty],
            TermNode::Data(DescriptorId::Struct(strukt)) => self
                .view(*strukt)
                .fields()
                .iter()
                .flat_map(|f| self.flatten_type(f.ty))
                .collect(),
            TermNode::Apply { target, .. } => self.flatten_type(*target),
            _ => vec![ty],
        }
    }
    pub fn view<T>(&self, data: T) -> HirViewer<'_, T> {
        HirViewer { hir: self, data }
    }
}
