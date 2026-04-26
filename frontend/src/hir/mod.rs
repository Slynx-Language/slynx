pub mod error;
mod helpers;
pub mod id;
mod implementation;
pub mod model;
pub mod modules;
pub mod names;

use std::collections::HashMap;

use crate::hir::{
    error::HIRError,
    model::{
        ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind, HirStatement,
        HirStatementKind, HirType,
    },
    modules::HirModules,
};
use common::{
    SymbolPointer,
    ast::{
        ASTDeclaration, ASTDeclarationKind, ASTStatementKind, ComponentMemberValue,
        VisibilityModifier,
    },
};

pub use id::{DeclarationId, ExpressionId, PropertyId, TypeId, VariableId};

pub type Result<T> = std::result::Result<T, HIRError>;

#[derive(Debug, Default)]
pub struct SlynxHir {
    modules: HirModules,
    /// Maps the types of top level things on the current scope to their types.
    /// An example is functions, which contain an HirType.
    types: HashMap<TypeId, HirType>,

    /// The scopes of this HIR. On the final it's expected to have only one, which is the global one
    pub declarations: Vec<HirDeclaration>,
}

impl SlynxHir {
    pub fn new() -> Self {
        Self {
            modules: HirModules::new(),
            types: HashMap::new(),
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

    /// Resolves the provided values on a component. The `ty` is the type of the component we are resolving it
    fn resolve_component_members(
        &mut self,
        members: Vec<ComponentMemberValue>,
        ty: TypeId,
    ) -> Result<Vec<ComponentMemberDeclaration>> {
        let mut out = Vec::with_capacity(members.len());
        for member in members {
            out.push(match member {
                ComponentMemberValue::Assign {
                    prop_name,
                    rhs,
                    span,
                } => {
                    let HirType::Component { props } = self.get_type(&ty) else {
                        unreachable!("The type should be a component instead");
                    };
                    let interned_name = self.modules.intern_name(&prop_name);
                    let index = props
                        .iter()
                        .position(|prop| prop.name() == prop_name)
                        .ok_or(HIRError::name_unrecognized(interned_name, span))?;

                    if matches!(
                        props[index].visibility(),
                        VisibilityModifier::Private | VisibilityModifier::ChildrenPublic
                    ) {
                        return Err(HIRError::not_visible_property(interned_name, span).into());
                    }
                    ComponentMemberDeclaration::Property {
                        id: PropertyId::new(), // Changed to PropertyId
                        index,
                        value: Some(self.resolve_expr(rhs, Some(*props[index].prop_type()))?),
                        span,
                    }
                }
                ComponentMemberValue::Child(child) => {
                    //By now this won't track whether it can or cannot have children, since a method better than 'children' might be implemented in the future.
                    {
                        let (id, _) = {
                            self.retrieve_information_of_type(
                                &child.name.identifier,
                                &child.name.span,
                            )?
                        };
                        let values = self.resolve_component_members(child.values, id)?;

                        ComponentMemberDeclaration::Child {
                            name: id,
                            values,
                            span: child.span,
                        }
                    }
                }
            });
        }
        Ok(out)
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
                    return Err(HIRError::name_unrecognized(symbol, ast.span).into());
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

                let alias_name = self.symbols_module.intern(&name.identifier);
                let Some(alias_ty) = self.types_module.get_type_from_name_mut(&alias_name) else {
                    return Err(HIRError::name_unrecognized(alias_name, name.span).into());
                };
                *alias_ty = HirType::Reference {
                    rf: target_ty,
                    generics: Vec::new(),
                };
                let (decl, ty) = if let Some(data) = self
                    .declarations_module
                    .retrieve_declaration_data_by_name(&alias_name)
                {
                    data
                } else {
                    return Err(HIRError::name_unrecognized(alias_name, name.span).into());
                };
                self.declarations.push(HirDeclaration {
                    id: decl,
                    kind: HirDeclarationKind::Alias,
                    ty,
                    span: ast.span,
                });
            }
        }
        Ok(())
    }
}
