pub mod declarations;
pub mod definitions;
pub mod error;
pub mod id; // New module for specific IDs
mod implementation;
pub mod names;
mod scopes;
pub mod symbols;
pub mod types;

use std::collections::HashMap;

use color_eyre::eyre::Result;

use crate::hir::{
    declarations::DeclarationsModule,
    definitions::{
        ComponentMemberDeclaration, HirDeclaration, HirDeclarationKind, HirStatement,
        HirStatementKind,
    },
    error::{HIRError, HIRErrorKind},
    scopes::ScopeModule,
    symbols::SymbolsModule,
    types::{BUILTIN_NAMES, HirType, TypesModule},
};
use common::ast::{
    ASTDeclaration, ASTDeclarationKind, ASTStatementKind, ComponentMemberKind,
    ComponentMemberValue, Span, VisibilityModifier,
};

// Re-export new ID types for convenience
pub use id::{DeclarationId, ExpressionId, PropertyId, TypeId, VariableId};
pub use symbols::SymbolPointer;

#[derive(Debug, Default)]
pub struct SlynxHir {
    ///The module that will keep track of all declarations on the top level
    pub declarations_module: DeclarationsModule,
    pub symbols_module: SymbolsModule,
    pub types_module: TypesModule,
    scope_module: ScopeModule,

    /// Maps the types of top level things on the current scope to their types.
    /// An example is functions, which contain an HirType.
    types: HashMap<TypeId, HirType>,
    /// Tracks the original source-level symbol for each variable id.
    variable_names: HashMap<VariableId, SymbolPointer>,

    /// The scopes of this HIR. On the final it's expected to have only one, which is the global one
    pub declarations: Vec<HirDeclaration>,
}

impl SlynxHir {
    pub fn new() -> Self {
        let mut symbols = SymbolsModule::new();
        let builtins = BUILTIN_NAMES.map(|v| symbols.intern(v));
        Self {
            symbols_module: symbols,
            scope_module: ScopeModule::new(),
            types: HashMap::new(),
            variable_names: HashMap::new(),
            declarations: Vec::new(),
            declarations_module: DeclarationsModule::new(),
            types_module: TypesModule::new(&builtins),
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

    /// Retrieves the id of the variable with the provided `name` going from the global scope, to the most internal one
    pub fn retrieve_variable_id(&mut self, name: &str, span: &Span) -> Result<VariableId> {
        let Some(symbol) = self.symbols_module.retrieve(name) else {
            return Err(HIRError {
                kind: HIRErrorKind::NameNotRecognized(name.to_string()),
                span: span.clone(),
            }
            .into());
        };
        let mut idx = self.scope_module.len() - 1;
        while idx != 0 {
            let scope = &self.scope_module[idx];

            let Some(id) = scope.retrieve_name(symbol) else {
                idx -= 1;
                continue;
            };
            return Ok(*id);
        }
        Err(HIRError {
            kind: HIRErrorKind::NameNotRecognized(name.to_string()),
            span: span.clone(),
        }
        .into())
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
                    let t = self.types_module.get_type(&ty);
                    let HirType::Component { props } = t else {
                        unreachable!("The type should be a component instead");
                    };
                    let index =
                        props
                            .iter()
                            .position(|prop| prop.1 == prop_name)
                            .ok_or(HIRError {
                                kind: HIRErrorKind::NameNotRecognized(prop_name.clone()),
                                span: span.clone(),
                            })?;

                    if matches!(
                        props[index].0,
                        VisibilityModifier::Private | VisibilityModifier::ChildrenPublic
                    ) {
                        return Err(HIRError {
                            kind: HIRErrorKind::PropertyNotVisible { prop_name },
                            span,
                        }
                        .into());
                    }
                    ComponentMemberDeclaration::Property {
                        id: PropertyId::new(), // Changed to PropertyId
                        index,
                        value: Some(self.resolve_expr(rhs, Some(props[index].2))?),
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
                self.symbols_module.intern(&target.identifier);
                let symbol = self.symbols_module.intern(&name.identifier);
                let ty = self.types_module.insert_type(symbol, HirType::Int);
                self.declarations_module.create_declaration(symbol, ty);
            }
            ASTDeclarationKind::ObjectDeclaration { name, fields } => {
                self.hoist_object(name, fields)?
            }

            ASTDeclarationKind::FuncDeclaration {
                name,
                args,
                return_type,
                ..
            } => self.hoist_function(name, args, return_type)?,
            ASTDeclarationKind::ComponentDeclaration { name, members, .. } => {
                let props = {
                    let mut out = Vec::with_capacity(members.len());
                    for member in members {
                        match &member.kind {
                            ComponentMemberKind::Property {
                                name, modifier, ty, ..
                            } => {
                                out.push((
                                    modifier.clone(),
                                    name.clone(),
                                    if let Some(generic) = ty {
                                        self.get_typeid_of_name(&generic.identifier, &member.span)?
                                    } else {
                                        self.types_module.infer_id()
                                    },
                                ));
                            }
                            ComponentMemberKind::Child(_) => {}
                        }
                    }
                    out
                };
                let symbol = self.symbols_module.intern(&name.identifier);
                let tyid = self
                    .types_module
                    .insert_type(symbol, HirType::Component { props });
                self.declarations_module.create_declaration(symbol, tyid);
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
            } => {
                let (decl, tyid, symb) = if let Some(symb) =
                    self.symbols_module.retrieve(&name.identifier)
                    && let Some(data) = self
                        .declarations_module
                        .retrieve_declaration_data_by_name(symb)
                {
                    let (decl, ty) = data;
                    (decl, ty, *symb)
                } else {
                    return Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.identifier),
                        span: name.span,
                    }
                    .into());
                };

                self.scope_module.enter_scope();

                let args = args
                    .into_iter()
                    .map(|arg| {
                        match self
                            .retrieve_information_of_type(&arg.kind.identifier, &arg.kind.span)
                        {
                            Ok((ty, _)) => Ok(self.create_variable(&arg.name, ty, false)),
                            Err(e) => Err(e),
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let mut statements = Vec::with_capacity(body.len());
                let body_len = body.len();
                for (index, statement) in body.into_iter().enumerate() {
                    let is_last = index + 1 == body_len;
                    match statement {
                        // The last expression in a function body becomes the implicit return.
                        common::ast::ASTStatement {
                            kind: ASTStatementKind::Expression(expr),
                            ..
                        } if is_last => {
                            let expr = self.resolve_expr(expr, None)?;
                            statements.push(HirStatement {
                                span: expr.span.clone(),
                                kind: HirStatementKind::Return { expr },
                            });
                        }
                        statement => statements.push(self.resolve_statement(statement)?),
                    }
                }

                self.declarations.push(HirDeclaration {
                    kind: HirDeclarationKind::Function {
                        statements,
                        args,
                        name: symb,
                    },
                    id: decl,
                    ty: tyid,
                    span: ast.span,
                });
                self.scope_module.exit_scope();
            }
            ASTDeclarationKind::ComponentDeclaration { members, name } => {
                self.scope_module.enter_scope();
                let symbol = self.symbols_module.intern(&name.identifier);
                let (decl, ty) = if let Some(data) = self
                    .declarations_module
                    .retrieve_declaration_data_by_name(&symbol)
                {
                    data
                } else {
                    return Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.identifier),
                        span: ast.span,
                    }
                    .into());
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
                self.scope_module.exit_scope();
            }
            ASTDeclarationKind::Alias { name, target } => {
                let target_ty = self.get_typeid_of_name(&target.identifier, &target.span)?;

                let alias_name = self.symbols_module.intern(&name.identifier);
                let Some(alias_ty) = self.types_module.get_type_from_name_mut(&alias_name) else {
                    return Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.identifier),
                        span: name.span,
                    }
                    .into());
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
                    return Err(HIRError {
                        kind: HIRErrorKind::NameNotRecognized(name.identifier),
                        span: name.span,
                    }
                    .into());
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
