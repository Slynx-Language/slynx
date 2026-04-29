pub mod error;
mod helpers;
pub mod id;
mod implementation;
pub mod model;
pub mod modules;
pub mod names;

use crate::hir::{
    error::HIRError,
    model::{HirDeclaration, HirDeclarationKind, HirType},
    modules::HirModules,
};
use common::{
    SymbolPointer,
    ast::{ASTDeclaration, ASTDeclarationKind},
};

pub use id::{DeclarationId, ExpressionId, PropertyId, TypeId, VariableId};

pub type Result<T> = std::result::Result<T, HIRError>;

#[derive(Debug, Default)]
pub struct SlynxHir {
    pub modules: HirModules,

    /// The scopes of this HIR. On the final it's expected to have only one, which is the global one
    pub declarations: Vec<HirDeclaration>,
}

impl SlynxHir {
    pub fn new() -> Self {
        Self {
            modules: HirModules::new(),
            declarations: Vec::new(),
        }
    }

    /// Generates the declarations from the provided `ast`
    pub fn generate(&mut self, ast: Vec<ASTDeclaration>) -> Result<()> {
        for ast in &ast {
            self.hoist(ast)?;
        }
        for ast in ast {
            self.resolve(ast)?;
        }
        Ok(())
    }

    /// Hoist the provided `ast` declaration, so no errors of undefined values because declared later may occur
    fn hoist(&mut self, ast: &ASTDeclaration) -> Result<()> {
        match &ast.kind {
            ASTDeclarationKind::Alias { name, target } => {
                self.modules
                    .create_alias(&target.identifier, &name.identifier);
            }
            ASTDeclarationKind::ObjectDeclaration { name, fields } => {
                self.modules.create_object(&name.identifier, fields)
            }

            ASTDeclarationKind::FuncDeclaration {
                name,
                args,
                return_type,
                ..
            } => self.hoist_function(name, args, return_type)?,
            ASTDeclarationKind::ComponentDeclaration { name, members, .. } => {
                self.hoist_component(name, members)?
            }
        }
        Ok(())
    }

    fn resolve(&mut self, ast: ASTDeclaration) -> Result<()> {
        match ast.kind {
            ASTDeclarationKind::ObjectDeclaration { name, fields } => {
                self.resolve_object(name, fields, ast.span)?
            }
            ASTDeclarationKind::FuncDeclaration {
                name, args, body, ..
            } => self.resolve_function(&name, &args, body, &ast.span)?,
            ASTDeclarationKind::ComponentDeclaration { members, name } => {
                self.modules.enter_scope();
                let symbol = self.modules.intern_name(&name.identifier);
                let Some((decl, ty)) = self.modules.get_declaration_by_name(&symbol) else {
                    return Err(HIRError::name_unrecognized(symbol, ast.span));
                };

                let defs = self.resolve_component_defs(members)?;
                self.declarations.push(HirDeclaration {
                    id: decl,
                    kind: HirDeclarationKind::ComponentDeclaration {
                        props: defs,
                        name: symbol,
                    },
                    ty,
                    span: ast.span,
                });
                self.modules.exit_scope();
            }
            ASTDeclarationKind::Alias { name, target } => {
                let target_ty = self.get_typeid_of_name(&target.identifier, &target.span)?;

                let alias_name = self.modules.intern_name(&name.identifier);
                let Some(alias_ty) = self
                    .modules
                    .types_module
                    .get_type_from_name_mut(&alias_name)
                else {
                    return Err(HIRError::name_unrecognized(alias_name, name.span));
                };
                *alias_ty = HirType::new_ref(target_ty);
                let Some((decl, ty)) = self.modules.get_declaration_by_name(&alias_name) else {
                    return Err(HIRError::name_unrecognized(alias_name, name.span));
                };
                self.declarations
                    .push(HirDeclaration::new_alias(decl, ty, ast.span));
            }
        }
        Ok(())
    }
}
