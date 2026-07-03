use std::ops::Deref;

use common::{
    Span,
    pool::{DedupPoolId, PoolId},
};
use dashmap::mapref::one::{Ref, RefMut};
use module_loader::FileId;

use crate::{
    DeclarationId, HIRError, HirComponentExpression, HirExpression, HirFunctionDeclaration,
    HirStatement, HirType, Result, SlynxHir, SymbolPointer,
    context::HirSymbol,
    file::HirFile,
    helpers::HirViewer,
    id::{AnyDeclarationId, AnyLocalDeclarationId},
};

impl SlynxHir<'_> {
    pub fn insert_expression(&self, expr: HirExpression) -> PoolId<HirExpression> {
        self.expressions.insert(expr)
    }
    pub fn insert_statement(&self, stmt: HirStatement) -> PoolId<HirStatement> {
        self.statements.insert(stmt)
    }
    pub fn insert_component_expression(
        &self,
        component: HirComponentExpression,
    ) -> PoolId<HirComponentExpression> {
        self.component_expressions.insert(component)
    }
    pub fn find_function_by_symbol(
        &self,
        symbol: HirSymbol,
    ) -> Option<DeclarationId<HirFunctionDeclaration>> {
        self.symbols_registry.get_function(symbol)
    }

    pub fn intern_name(&self, name: &str) -> SymbolPointer {
        self.symbols_resolver.intern(name)
    }

    pub fn get_name(&self, name: SymbolPointer) -> &str {
        self.symbols_resolver.get_name(name)
    }

    pub fn get_file(&self, id: FileId) -> Ref<'_, FileId, HirFile> {
        self.files
            .get(&id)
            .expect("A file with the given id should exist")
    }
    pub fn get_file_mut(&self, id: FileId) -> RefMut<'_, FileId, HirFile> {
        self.files
            .get_mut(&id)
            .expect("A file with the given id should exist")
    }
    pub fn get_declaration_type(&self, id: AnyDeclarationId) -> DedupPoolId<HirType> {
        let file = self.get_or_create_file(id.file_id);
        match id.local_id {
            AnyLocalDeclarationId::Alias(alias) => file.alias.get(alias).ty,
            AnyLocalDeclarationId::Component(component) => file.components.get(component).ty,
            AnyLocalDeclarationId::Function(func) => file.functions.get(func).ty,
            AnyLocalDeclarationId::Object(obj) => file.objects.get(obj).ty,
            AnyLocalDeclarationId::Static(statik) => file.statik.get(statik).ty,
            AnyLocalDeclarationId::Style(style) => file.styles.get(style).ty,
        }
    }

    pub fn type_of_intrinsic(&self, name: &str, span: Span) -> Result<DedupPoolId<HirType>> {
        let id = self.lang_items.get(name).map_err(|_| {
            let sym = self.intern_name(name);
            HIRError::intrinsic_not_registered(sym, span)
        })?;
        Ok(self.get_declaration_type(id))
    }

    /// Recursively flattens a HIR type to its primitive components.
    /// A struct `Color { inner: int }` flattens to `[int]`.
    /// A struct `Border { color: Color, width: int, radius: int }` flattens to `[int, int, int]`.
    pub fn flatten_type(&self, ty: DedupPoolId<HirType>) -> Vec<DedupPoolId<HirType>> {
        match &self.deref()[ty] {
            HirType::Int | HirType::Float | HirType::Bool | HirType::Str => vec![ty],
            HirType::Struct(strukt) => self
                .view(*strukt)
                .field_types()
                .iter()
                .flat_map(|f| self.flatten_type(*f))
                .collect(),
            HirType::Reference { rf, .. } => self.flatten_type(*rf),
            _ => vec![ty],
        }
    }
    pub fn view<T>(&self, data: T) -> HirViewer<'_, T> {
        HirViewer { hir: self, data }
    }
}
