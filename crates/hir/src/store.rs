//! The HIR's data layer: every pool and map that stores HIR data.

use std::ops::Index;

use common::pool::{Pool, PoolId};
use dashmap::{DashMap, mapref::one::Ref, mapref::one::RefMut};
use module_loader::FileId;

use crate::{
    HirComponentExpression, HirExpression, HirPlace, HirStatement, SymbolPointer, VariableId,
    context::LangItems, file::HirFile,
};

#[derive(Debug)]
/// All data stored by the HIR: the expression/statement/component-expression/
/// place pools, the files containing declarations, the variable-name map, and
/// the language items table.
///
/// This is pure data storage. It knows nothing about types ([`crate::TypesContext`]),
/// symbols ([`crate::context::SymbolRegistry`]) or how the HIR gets built.
pub struct HirStore {
    pub expressions: Pool<HirExpression>,
    pub statements: Pool<HirStatement>,
    pub component_expressions: Pool<HirComponentExpression>,
    /// Pool of places constructed during ownership analysis.
    pub places: Pool<HirPlace>,
    /// Mapping from VariableId to its source name, populated during HIR construction.
    pub variable_names: DashMap<VariableId, SymbolPointer>,
    /// All top-level declarations generated from the sources.
    ///
    /// This map contains every function, component, object, and type alias
    /// defined in the source code.
    pub files: DashMap<FileId, HirFile>,
    pub lang_items: LangItems,
}
impl Default for HirStore {
    fn default() -> Self {
        Self::new()
    }
}
impl HirStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self {
            expressions: Pool::new(),
            statements: Pool::new(),
            component_expressions: Pool::new(),
            places: Pool::new(),
            variable_names: DashMap::new(),
            files: DashMap::new(),
            lang_items: LangItems::new(),
        }
    }

    ///Inserts an expression into the expression pool and returns its id.
    pub fn insert_expression(&self, expr: HirExpression) -> PoolId<HirExpression> {
        self.expressions.insert(expr)
    }

    ///Inserts a statement into the statement pool and returns its id.
    pub fn insert_statement(&self, stmt: HirStatement) -> PoolId<HirStatement> {
        self.statements.insert(stmt)
    }

    ///Inserts a component expression into its pool and returns its id.
    pub fn insert_component_expression(
        &self,
        component: HirComponentExpression,
    ) -> PoolId<HirComponentExpression> {
        self.component_expressions.insert(component)
    }

    ///Gets or create an Hir file with the given `id`
    pub fn get_or_create_file(&self, id: FileId) -> RefMut<'_, FileId, HirFile> {
        self.files.entry(id).or_insert_with(|| HirFile::new(id))
    }

    ///Gets the file with the given `id`, panics if it does not exist.
    pub fn get_file(&self, id: FileId) -> Ref<'_, FileId, HirFile> {
        self.files
            .get(&id)
            .expect("A file with the given id should exist")
    }

    ///Gets a mutable reference to the file with the given `id`, panics if it does not exist.
    pub fn get_file_mut(&self, id: FileId) -> RefMut<'_, FileId, HirFile> {
        self.files
            .get_mut(&id)
            .expect("A file with the given id should exist")
    }
}

impl Index<PoolId<HirExpression>> for HirStore {
    type Output = HirExpression;
    fn index(&self, index: PoolId<HirExpression>) -> &Self::Output {
        &self.expressions[index]
    }
}

impl Index<PoolId<HirStatement>> for HirStore {
    type Output = HirStatement;
    fn index(&self, index: PoolId<HirStatement>) -> &Self::Output {
        &self.statements[index]
    }
}

impl Index<PoolId<HirComponentExpression>> for HirStore {
    type Output = HirComponentExpression;
    fn index(&self, index: PoolId<HirComponentExpression>) -> &Self::Output {
        &self.component_expressions[index]
    }
}

impl Index<PoolId<HirPlace>> for HirStore {
    type Output = HirPlace;
    fn index(&self, index: PoolId<HirPlace>) -> &Self::Output {
        &self.places[index]
    }
}
